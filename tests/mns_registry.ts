import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MnsRegistry } from "../target/types/mns_registry";
import { expect } from "chai";

describe("mns_registry", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MnsRegistry as Program<MnsRegistry>;
  
  const [registryPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("registry")],
    program.programId
  );

  it("initializes the registry", async () => {
    const tx = await program.methods
      .initialize()
      .accounts({
        authority: provider.wallet.publicKey,
      })
      .rpc();
    
    const registry = await program.account.registry.fetch(registryPda);
    expect(registry.totalRegistered.toNumber()).to.equal(0);
    expect(registry.feeLamports.toNumber()).to.equal(100_000_000);
  });

  it("registers a name", async () => {
    const name = "testname";
    const [namePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("name"), Buffer.from(name)],
      program.programId
    );

    const treasury = anchor.web3.Keypair.generate();

    const tx = await program.methods
      .registerName(name)
      .accounts({
        owner: provider.wallet.publicKey,
        treasury: treasury.publicKey,
      })
      .rpc();

    const nameRecord = await program.account.nameRecord.fetch(namePda);
    expect(nameRecord.name).to.equal(name);
    expect(nameRecord.owner.toBase58()).to.equal(provider.wallet.publicKey.toBase58());
  });

  it("rejects invalid name length", async () => {
    const name = "ab"; // Too short
    
    try {
      await program.methods
        .registerName(name)
        .accounts({
          owner: provider.wallet.publicKey,
          treasury: anchor.web3.Keypair.generate().publicKey,
        })
        .rpc();
      expect.fail("Should have thrown");
    } catch (e) {
      expect(e.error.errorCode.code).to.equal("InvalidNameLength");
    }
  });

  it("transfers name ownership", async () => {
    const name = "transfer";
    const [namePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("name"), Buffer.from(name)],
      program.programId
    );

    const treasury = anchor.web3.Keypair.generate();
    const newOwner = anchor.web3.Keypair.generate();

    // Register
    await program.methods
      .registerName(name)
      .accounts({
        owner: provider.wallet.publicKey,
        treasury: treasury.publicKey,
      })
      .rpc();

    // Transfer
    await program.methods
      .transferName(name)
      .accounts({
        owner: provider.wallet.publicKey,
        newOwner: newOwner.publicKey,
      })
      .rpc();

    const nameRecord = await program.account.nameRecord.fetch(namePda);
    expect(nameRecord.owner.toBase58()).to.equal(newOwner.publicKey.toBase58());
  });

  it("renews a name", async () => {
    const name = "renewable";
    const [namePda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("name"), Buffer.from(name)],
      program.programId
    );

    const treasury = anchor.web3.Keypair.generate();

    // Register
    await program.methods
      .registerName(name)
      .accounts({
        owner: provider.wallet.publicKey,
        treasury: treasury.publicKey,
      })
      .rpc();

    const beforeRecord = await program.account.nameRecord.fetch(namePda);
    const beforeExpiry = beforeRecord.expiresAt.toNumber();

    // Renew for 2 years
    await program.methods
      .renewName(name, 2)
      .accounts({
        owner: provider.wallet.publicKey,
        treasury: treasury.publicKey,
      })
      .rpc();

    const afterRecord = await program.account.nameRecord.fetch(namePda);
    const afterExpiry = afterRecord.expiresAt.toNumber();

    const twoYearsInSeconds = 2 * 365 * 24 * 60 * 60;
    expect(afterExpiry - beforeExpiry).to.equal(twoYearsInSeconds);
  });
});


