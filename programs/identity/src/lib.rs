//! Altheia Identity Program
//!
//! Slim on-chain registry for AI agent identities + audit Merkle anchors.
//! Does NOT enforce token transfers, hold session keys, or own custody —
//! all on-chain enforcement happens at the Swig substrate layer.
//!
//! See altheia-plan/02_SRS/SMART_CONTRACT_SRS.md for the full spec.

use anchor_lang::prelude::*;

declare_id!("AkKx54ZmuP17r1sXsKr7mxe3dXJ5RMqsSH2zf8QGZ39C");

#[program]
pub mod identity {
    use super::*;

    /// Initialize an operator account. Called once per wallet.
    pub fn initialize_operator(ctx: Context<InitializeOperator>) -> Result<()> {
        let operator = &mut ctx.accounts.operator;
        operator.authority = ctx.accounts.authority.key();
        operator.created_at = Clock::get()?.unix_timestamp;
        operator.agent_count = 0;
        operator.active_agent_count = 0;
        operator.last_audit_root = [0u8; 32];
        operator.last_audit_anchored_at = 0;
        Ok(())
    }

    /// Register a new agent under the calling operator. Policy is stored
    /// in full on-chain — caller passes structured caps + program scope +
    /// blocked destinations. The off-chain `policy_commitment` hash is
    /// retained as a tamper-evidence aid (chain attests both plaintext +
    /// hash, anyone can verify hash(plaintext) == commitment).
    pub fn register_agent(
        ctx: Context<RegisterAgent>,
        agent_id: [u8; 32],
        framework: u8,
        model_commitment: [u8; 32],
        policy_commitment: [u8; 32],
        swig_account: Pubkey,
        asset_caps: Vec<TokenCap>,
        allowed_programs: Vec<Pubkey>,
        blocked_destinations: Vec<Pubkey>,
    ) -> Result<()> {
        require!(framework < 6, AltheiaError::InvalidFramework);
        require!(asset_caps.len() <= 4, AltheiaError::TooManyAssetCaps);
        require!(allowed_programs.len() <= 8, AltheiaError::TooManyAllowedPrograms);
        require!(blocked_destinations.len() <= 4, AltheiaError::TooManyBlockedDestinations);

        let agent = &mut ctx.accounts.agent;
        let operator = &mut ctx.accounts.operator;

        agent.operator = operator.authority;
        agent.agent_id = agent_id;
        agent.framework = framework;
        agent.model_commitment = model_commitment;
        agent.policy_commitment = policy_commitment;
        agent.swig_account = swig_account;
        agent.created_at = Clock::get()?.unix_timestamp;
        agent.last_updated_at = agent.created_at;
        agent.status = AgentStatus::Active;
        agent.revoked_at = None;
        agent.asset_caps = asset_caps;
        agent.allowed_programs = allowed_programs;
        agent.blocked_destinations = blocked_destinations;

        operator.agent_count = operator.agent_count.checked_add(1).ok_or(AltheiaError::Overflow)?;
        operator.active_agent_count = operator.active_agent_count.checked_add(1).ok_or(AltheiaError::Overflow)?;

        emit!(AgentRegistered {
            operator: operator.authority,
            agent: agent.key(),
            framework,
            swig_account,
            timestamp: agent.created_at,
        });

        Ok(())
    }

    /// Update the full on-chain policy: caps, allowed programs, blocked
    /// destinations, and the convenience hash. Since AgentAccount is
    /// allocated at max size at registration (Vec<T, max_len=N> reserves
    /// the worst-case bytes), this requires no realloc — we just rewrite
    /// the Vec contents within the existing allocation.
    pub fn update_policy(
        ctx: Context<ManageAgent>,
        new_policy_commitment: [u8; 32],
        new_asset_caps: Vec<TokenCap>,
        new_allowed_programs: Vec<Pubkey>,
        new_blocked_destinations: Vec<Pubkey>,
    ) -> Result<()> {
        require!(new_asset_caps.len() <= 4, AltheiaError::TooManyAssetCaps);
        require!(new_allowed_programs.len() <= 8, AltheiaError::TooManyAllowedPrograms);
        require!(new_blocked_destinations.len() <= 4, AltheiaError::TooManyBlockedDestinations);

        let agent = &mut ctx.accounts.agent;
        require!(
            agent.status == AgentStatus::Active || agent.status == AgentStatus::Paused,
            AltheiaError::InvalidStatusTransition
        );
        let old = agent.policy_commitment;
        agent.policy_commitment = new_policy_commitment;
        agent.asset_caps = new_asset_caps;
        agent.allowed_programs = new_allowed_programs;
        agent.blocked_destinations = new_blocked_destinations;
        agent.last_updated_at = Clock::get()?.unix_timestamp;

        emit!(PolicyUpdated {
            agent: agent.key(),
            old_commitment: old,
            new_commitment: new_policy_commitment,
            timestamp: agent.last_updated_at,
        });

        Ok(())
    }

    pub fn pause_agent(ctx: Context<ManageAgent>) -> Result<()> {
        let agent = &mut ctx.accounts.agent;
        require!(agent.status == AgentStatus::Active, AltheiaError::InvalidStatusTransition);

        agent.status = AgentStatus::Paused;
        agent.last_updated_at = Clock::get()?.unix_timestamp;

        emit!(AgentPaused {
            agent: agent.key(),
            timestamp: agent.last_updated_at,
        });

        Ok(())
    }

    pub fn unpause_agent(ctx: Context<ManageAgent>) -> Result<()> {
        let agent = &mut ctx.accounts.agent;
        require!(agent.status == AgentStatus::Paused, AltheiaError::InvalidStatusTransition);

        agent.status = AgentStatus::Active;
        agent.last_updated_at = Clock::get()?.unix_timestamp;

        emit!(AgentUnpaused {
            agent: agent.key(),
            timestamp: agent.last_updated_at,
        });

        Ok(())
    }

    pub fn revoke_agent(ctx: Context<RevokeAgent>, reason_code: u8) -> Result<()> {
        let agent = &mut ctx.accounts.agent;
        let operator = &mut ctx.accounts.operator;

        require!(
            agent.status == AgentStatus::Active || agent.status == AgentStatus::Paused,
            AltheiaError::InvalidStatusTransition
        );

        let now = Clock::get()?.unix_timestamp;
        agent.status = AgentStatus::Revoked;
        agent.revoked_at = Some(now);
        agent.last_updated_at = now;

        operator.active_agent_count = operator
            .active_agent_count
            .checked_sub(1)
            .ok_or(AltheiaError::Overflow)?;

        emit!(AgentRevoked {
            operator: operator.authority,
            agent: agent.key(),
            reason_code,
            timestamp: now,
        });

        Ok(())
    }

    pub fn archive_agent(ctx: Context<ManageAgent>) -> Result<()> {
        let agent = &mut ctx.accounts.agent;
        require!(agent.status == AgentStatus::Revoked, AltheiaError::InvalidStatusTransition);

        agent.status = AgentStatus::Archived;
        agent.last_updated_at = Clock::get()?.unix_timestamp;

        emit!(AgentArchived {
            agent: agent.key(),
            timestamp: agent.last_updated_at,
        });

        Ok(())
    }

    pub fn commit_audit_root(
        ctx: Context<CommitAuditRoot>,
        merkle_root: [u8; 32],
        period_start: i64,
        period_end: i64,
    ) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(period_start < period_end, AltheiaError::InvalidPeriod);
        require!(period_end <= now, AltheiaError::FuturePeriod);

        let operator = &mut ctx.accounts.operator;
        operator.last_audit_root = merkle_root;
        operator.last_audit_anchored_at = now;

        emit!(AuditRootCommitted {
            operator: operator.authority,
            merkle_root,
            period_start,
            period_end,
        });

        Ok(())
    }
}

// ─── Account structs ───────────────────────────────────────────────

#[account]
#[derive(InitSpace)]
pub struct OperatorAccount {
    pub authority: Pubkey,
    pub created_at: i64,
    pub agent_count: u32,
    pub active_agent_count: u32,
    pub last_audit_root: [u8; 32],
    pub last_audit_anchored_at: i64,
}

#[account]
#[derive(InitSpace)]
pub struct AgentAccount {
    pub operator: Pubkey,
    pub agent_id: [u8; 32],
    pub framework: u8,                   // 0=Eliza, 1=Virtuals, 2=Griffain, 3=SAK, 4=MCP, 5=Custom
    pub model_commitment: [u8; 32],
    pub policy_commitment: [u8; 32],     // sha256(canonical(asset_caps + allowed_programs + blocked_destinations))
    pub swig_account: Pubkey,            // reference to operator's Swig smart account
    pub created_at: i64,
    pub last_updated_at: i64,
    pub status: AgentStatus,
    pub revoked_at: Option<i64>,

    // ─── Full on-chain policy (replaces the old hash-only commitment) ───
    // The AgentAccount now stores the literal policy. Anyone reading the
    // account (Solscan, getAccountInfo, the SDK) sees exactly what the
    // chain enforces. policy_commitment above is kept as a redundant
    // tamper-evidence aid: hash(canonical(below)) MUST equal it.
    #[max_len(4)]
    pub asset_caps: Vec<TokenCap>,
    #[max_len(8)]
    pub allowed_programs: Vec<Pubkey>,
    #[max_len(4)]
    pub blocked_destinations: Vec<Pubkey>,
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TokenCap {
    pub mint: Pubkey,                    // SPL mint (or wrapped-SOL sentinel So111…112)
    pub max_per_tx: u64,                 // smallest unit of the token (lamports / micro-USDC)
    pub max_per_day: u64,                // smallest unit of the token
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    Active,
    Paused,
    Revoked,
    Archived,
}

// ─── Instruction contexts ──────────────────────────────────────────

#[derive(Accounts)]
pub struct InitializeOperator<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + OperatorAccount::INIT_SPACE,
        seeds = [b"operator", authority.key().as_ref()],
        bump
    )]
    pub operator: Account<'info, OperatorAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(agent_id: [u8; 32])]
pub struct RegisterAgent<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + AgentAccount::INIT_SPACE,
        seeds = [b"agent", operator.key().as_ref(), agent_id.as_ref()],
        bump
    )]
    pub agent: Account<'info, AgentAccount>,

    #[account(
        mut,
        seeds = [b"operator", authority.key().as_ref()],
        bump,
        has_one = authority,
    )]
    pub operator: Account<'info, OperatorAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManageAgent<'info> {
    #[account(
        mut,
        constraint = agent.operator == authority.key() @ AltheiaError::Unauthorized,
    )]
    pub agent: Account<'info, AgentAccount>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct RevokeAgent<'info> {
    #[account(
        mut,
        constraint = agent.operator == authority.key() @ AltheiaError::Unauthorized,
    )]
    pub agent: Account<'info, AgentAccount>,

    #[account(
        mut,
        seeds = [b"operator", authority.key().as_ref()],
        bump,
        has_one = authority,
    )]
    pub operator: Account<'info, OperatorAccount>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CommitAuditRoot<'info> {
    #[account(
        mut,
        seeds = [b"operator", authority.key().as_ref()],
        bump,
        has_one = authority,
    )]
    pub operator: Account<'info, OperatorAccount>,

    pub authority: Signer<'info>,
}

// ─── Events ────────────────────────────────────────────────────────

#[event]
pub struct AgentRegistered {
    pub operator: Pubkey,
    pub agent: Pubkey,
    pub framework: u8,
    pub swig_account: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct PolicyUpdated {
    pub agent: Pubkey,
    pub old_commitment: [u8; 32],
    pub new_commitment: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct AgentPaused {
    pub agent: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AgentUnpaused {
    pub agent: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AgentRevoked {
    pub operator: Pubkey,
    pub agent: Pubkey,
    pub reason_code: u8,
    pub timestamp: i64,
}

#[event]
pub struct AgentArchived {
    pub agent: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct AuditRootCommitted {
    pub operator: Pubkey,
    pub merkle_root: [u8; 32],
    pub period_start: i64,
    pub period_end: i64,
}

// ─── Errors ────────────────────────────────────────────────────────

#[error_code]
pub enum AltheiaError {
    #[msg("Invalid framework enum (must be 0-5)")]
    InvalidFramework,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Invalid status transition for current agent state")]
    InvalidStatusTransition,
    #[msg("Caller is not authorized to operate on this agent")]
    Unauthorized,
    #[msg("Audit period must satisfy start < end")]
    InvalidPeriod,
    #[msg("Audit period_end cannot be in the future")]
    FuturePeriod,
    #[msg("Too many asset caps (max 4 per agent)")]
    TooManyAssetCaps,
    #[msg("Too many allowed programs (max 8 per agent)")]
    TooManyAllowedPrograms,
    #[msg("Too many blocked destinations (max 4 per agent)")]
    TooManyBlockedDestinations,
}

// Make authority field accessible for `has_one` constraint
impl OperatorAccount {
    pub const fn authority(&self) -> &Pubkey {
        &self.authority
    }
}
