//! Altheia Identity Program
//!
//! Slim on-chain registry for AI agent identities + audit Merkle anchors.
//! Does NOT enforce token transfers, hold session keys, or own custody —
//! all on-chain enforcement happens at the Swig substrate layer.
//!
//! See altheia-plan/02_SRS/SMART_CONTRACT_SRS.md for the full spec.

use anchor_lang::prelude::*;

declare_id!("AthIdentity1111111111111111111111111111111");

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

    /// Register a new agent under the calling operator.
    pub fn register_agent(
        ctx: Context<RegisterAgent>,
        agent_id: [u8; 32],
        framework: u8,
        model_commitment: [u8; 32],
        policy_commitment: [u8; 32],
        swig_account: Pubkey,
    ) -> Result<()> {
        require!(framework < 6, AltheiaError::InvalidFramework);

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

    // TODO: update_policy_commitment, pause_agent, unpause_agent,
    //       revoke_agent, archive_agent, commit_audit_root
    //       (see SMART_CONTRACT_SRS.md §1)
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
    pub policy_commitment: [u8; 32],
    pub swig_account: Pubkey,            // reference to operator's Swig smart account
    pub created_at: i64,
    pub last_updated_at: i64,
    pub status: AgentStatus,
    pub revoked_at: Option<i64>,
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

// ─── Events ────────────────────────────────────────────────────────

#[event]
pub struct AgentRegistered {
    pub operator: Pubkey,
    pub agent: Pubkey,
    pub framework: u8,
    pub swig_account: Pubkey,
    pub timestamp: i64,
}

// ─── Errors ────────────────────────────────────────────────────────

#[error_code]
pub enum AltheiaError {
    #[msg("Invalid framework enum (must be 0-5)")]
    InvalidFramework,
    #[msg("Arithmetic overflow")]
    Overflow,
}

// Make authority field accessible for `has_one` constraint
impl OperatorAccount {
    pub const fn authority(&self) -> &Pubkey {
        &self.authority
    }
}
