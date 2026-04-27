import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Identity } from "../target/types/identity";
import { expect } from "chai";

describe("identity", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.identity as Program<Identity>;

  it("initializes an operator", async () => {
    const [operatorPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("operator"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    await program.methods
      .initializeOperator()
      .accounts({
        operator: operatorPda,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const operator = await program.account.operatorAccount.fetch(operatorPda);
    expect(operator.authority.toBase58()).to.equal(
      provider.wallet.publicKey.toBase58()
    );
    expect(operator.agentCount).to.equal(0);
    expect(operator.activeAgentCount).to.equal(0);
  });

  // TODO: register_agent test
  // TODO: update_policy_commitment test
  // TODO: revoke_agent test
});
