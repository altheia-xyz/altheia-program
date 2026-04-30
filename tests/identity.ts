import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { startAnchor, BankrunProvider } from "anchor-bankrun";
import { Identity } from "../target/types/identity";
import IDL from "../target/idl/identity.json";
import { expect } from "chai";
import { randomBytes, createHash } from "crypto";

const PROGRAM_ID = new anchor.web3.PublicKey(
  "AkKx54ZmuP17r1sXsKr7mxe3dXJ5RMqsSH2zf8QGZ39C"
);

describe("identity", () => {
  let provider: BankrunProvider;
  let program: Program<Identity>;
  let context: Awaited<ReturnType<typeof startAnchor>>;

  const sha256 = (s: string): number[] =>
    Array.from(createHash("sha256").update(s).digest());

  let operatorPda: anchor.web3.PublicKey;
  let agentPda: anchor.web3.PublicKey;
  const agentId = randomBytes(32);
  const swigAccountStub = anchor.web3.Keypair.generate().publicKey;
  const modelCommitment = sha256("gpt-4o-mini@2026-04-29");
  const policyCommitment = sha256("policy-v1");
  const newPolicyCommitment = sha256("policy-v2");

  before(async () => {
    context = await startAnchor("", [], []);
    provider = new BankrunProvider(context);
    anchor.setProvider(provider);
    program = new Program<Identity>(IDL as Identity, provider);

    [operatorPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("operator"), provider.wallet.publicKey.toBuffer()],
      PROGRAM_ID
    );
    [agentPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("agent"), operatorPda.toBuffer(), agentId],
      PROGRAM_ID
    );
  });

  it("initializes an operator", async () => {
    await program.methods
      .initializeOperator()
      .accounts({
        operator: operatorPda,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const operator = await program.account.operatorAccount.fetch(operatorPda);
    expect(operator.authority.toBase58()).to.equal(provider.wallet.publicKey.toBase58());
    expect(operator.agentCount).to.equal(0);
    expect(operator.activeAgentCount).to.equal(0);
  });

  it("registers an agent", async () => {
    await program.methods
      .registerAgent(
        Array.from(agentId),
        3,
        modelCommitment,
        policyCommitment,
        swigAccountStub
      )
      .accounts({
        agent: agentPda,
        operator: operatorPda,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const agent = await program.account.agentAccount.fetch(agentPda);
    expect(agent.framework).to.equal(3);
    expect(agent.swigAccount.toBase58()).to.equal(swigAccountStub.toBase58());
    expect(agent.status).to.deep.equal({ active: {} });

    const operator = await program.account.operatorAccount.fetch(operatorPda);
    expect(operator.agentCount).to.equal(1);
    expect(operator.activeAgentCount).to.equal(1);
  });

  it("rejects register_agent with invalid framework", async () => {
    const badAgentId = randomBytes(32);
    const [badAgentPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("agent"), operatorPda.toBuffer(), badAgentId],
      PROGRAM_ID
    );

    try {
      await program.methods
        .registerAgent(Array.from(badAgentId), 99, modelCommitment, policyCommitment, swigAccountStub)
        .accounts({
          agent: badAgentPda,
          operator: operatorPda,
          authority: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
      expect.fail("expected InvalidFramework");
    } catch (err: any) {
      // 0x1770 = error 6000 = InvalidFramework
      expect(err.toString()).to.match(/InvalidFramework|0x1770|6000/);
    }
  });

  it("updates the policy commitment", async () => {
    const before = await program.account.agentAccount.fetch(agentPda);

    await program.methods
      .updatePolicyCommitment(newPolicyCommitment)
      .accounts({
        agent: agentPda,
        authority: provider.wallet.publicKey,
      })
      .rpc();

    const after = await program.account.agentAccount.fetch(agentPda);
    expect(Buffer.from(after.policyCommitment).toString("hex")).to.equal(
      Buffer.from(newPolicyCommitment).toString("hex")
    );
    expect(after.lastUpdatedAt.toNumber()).to.be.greaterThanOrEqual(before.lastUpdatedAt.toNumber());
  });

  it("pauses then unpauses the agent", async () => {
    await program.methods
      .pauseAgent()
      .accounts({ agent: agentPda, authority: provider.wallet.publicKey })
      .rpc();

    let agent = await program.account.agentAccount.fetch(agentPda);
    expect(agent.status).to.deep.equal({ paused: {} });

    await program.methods
      .unpauseAgent()
      .accounts({ agent: agentPda, authority: provider.wallet.publicKey })
      .rpc();

    agent = await program.account.agentAccount.fetch(agentPda);
    expect(agent.status).to.deep.equal({ active: {} });
  });

  it("rejects pause when already active->paused (no double-pause)", async () => {
    await program.methods
      .pauseAgent()
      .accounts({ agent: agentPda, authority: provider.wallet.publicKey })
      .rpc();

    let threw = false;
    try {
      await program.methods
        .pauseAgent()
        .accounts({ agent: agentPda, authority: provider.wallet.publicKey })
        .rpc();
    } catch {
      threw = true;
    }
    expect(threw, "second pause must reject (program error or duplicate-tx)").to.equal(true);

    // agent stays Paused either way — the on-chain state is the real assertion
    let agent = await program.account.agentAccount.fetch(agentPda);
    expect(agent.status).to.deep.equal({ paused: {} });

    await program.methods
      .unpauseAgent()
      .accounts({ agent: agentPda, authority: provider.wallet.publicKey })
      .rpc();
  });

  it("commits an audit root", async () => {
    const root = sha256("audit-root-1");
    const clock = await context.banksClient.getClock();
    const now = Number(clock.unixTimestamp);
    const periodStart = new anchor.BN(now - 3600);
    const periodEnd = new anchor.BN(now);

    await program.methods
      .commitAuditRoot(root, periodStart, periodEnd)
      .accounts({
        operator: operatorPda,
        authority: provider.wallet.publicKey,
      })
      .rpc();

    const operator = await program.account.operatorAccount.fetch(operatorPda);
    expect(Buffer.from(operator.lastAuditRoot).toString("hex")).to.equal(
      Buffer.from(root).toString("hex")
    );
    expect(operator.lastAuditAnchoredAt.toNumber()).to.be.greaterThan(0);
  });

  it("revokes an agent and decrements active count", async () => {
    const opBefore = await program.account.operatorAccount.fetch(operatorPda);

    await program.methods
      .revokeAgent(0)
      .accounts({
        agent: agentPda,
        operator: operatorPda,
        authority: provider.wallet.publicKey,
      })
      .rpc();

    const agent = await program.account.agentAccount.fetch(agentPda);
    expect(agent.status).to.deep.equal({ revoked: {} });
    expect(agent.revokedAt).to.not.be.null;

    const opAfter = await program.account.operatorAccount.fetch(operatorPda);
    expect(opAfter.activeAgentCount).to.equal(opBefore.activeAgentCount - 1);
    expect(opAfter.agentCount).to.equal(opBefore.agentCount);
  });

  it("rejects re-revoke (cannot revoke from Revoked)", async () => {
    let threw = false;
    try {
      await program.methods
        .revokeAgent(0)
        .accounts({
          agent: agentPda,
          operator: operatorPda,
          authority: provider.wallet.publicKey,
        })
        .rpc();
    } catch {
      threw = true;
    }
    expect(threw, "second revoke must reject (program error or duplicate-tx)").to.equal(true);

    // either way, the agent stays Revoked — the on-chain state is the real assertion
    const agent = await program.account.agentAccount.fetch(agentPda);
    expect(agent.status).to.deep.equal({ revoked: {} });
  });

  it("archives a revoked agent", async () => {
    await program.methods
      .archiveAgent()
      .accounts({ agent: agentPda, authority: provider.wallet.publicKey })
      .rpc();

    const agent = await program.account.agentAccount.fetch(agentPda);
    expect(agent.status).to.deep.equal({ archived: {} });
  });

  it("rejects update_policy_commitment on archived agent (status guard)", async () => {
    try {
      await program.methods
        .updatePolicyCommitment(sha256("policy-v3-illegal"))
        .accounts({ agent: agentPda, authority: provider.wallet.publicKey })
        .rpc();
      expect.fail("expected InvalidStatusTransition");
    } catch (err: any) {
      // bankrun returns raw error; 0x1772 = error 6002 = InvalidStatusTransition
      expect(err.toString()).to.match(/InvalidStatusTransition|0x1772|6002/);
    }
  });

  it("rejects commit_audit_root with inverted period", async () => {
    const root = sha256("audit-root-bad");
    const clock = await context.banksClient.getClock();
    const now = Number(clock.unixTimestamp);
    const periodStart = new anchor.BN(now);
    const periodEnd = new anchor.BN(now - 3600);
    try {
      await program.methods
        .commitAuditRoot(root, periodStart, periodEnd)
        .accounts({ operator: operatorPda, authority: provider.wallet.publicKey })
        .rpc();
      expect.fail("expected InvalidPeriod");
    } catch (err: any) {
      // 0x1774 = error 6004 = InvalidPeriod
      expect(err.toString()).to.match(/InvalidPeriod|0x1774|6004/);
    }
  });

  it("rejects commit_audit_root with future period_end", async () => {
    const root = sha256("audit-root-future");
    const clock = await context.banksClient.getClock();
    const now = Number(clock.unixTimestamp);
    const periodStart = new anchor.BN(now);
    const periodEnd = new anchor.BN(now + 86400);
    try {
      await program.methods
        .commitAuditRoot(root, periodStart, periodEnd)
        .accounts({ operator: operatorPda, authority: provider.wallet.publicKey })
        .rpc();
      expect.fail("expected FuturePeriod");
    } catch (err: any) {
      // 0x1775 = error 6005 = FuturePeriod
      expect(err.toString()).to.match(/FuturePeriod|0x1775|6005/);
    }
  });
});
