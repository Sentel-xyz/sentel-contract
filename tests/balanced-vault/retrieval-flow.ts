/**
 * Balanced Vault Retrieval Flow Tests
 *
 * Covers the complete propose -> approve -> execute lifecycle for
 * retrieve_transaction, plus close_zombie_retrieve guard checks.
 *
 * Note: execute_retrieve_transaction contains a Jupiter CPI path for
 * token-to-WSOL swaps. When the vault holds only WSOL (no other tokens),
 * the instruction skips Jupiter and unwraps WSOL directly to SOL. That
 * path is what this test exercises on localnet.
 */

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { SentelContract } from "../target/types/sentel_contract";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createInitializeAccountInstruction,
  getAssociatedTokenAddressSync,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { expect } from "chai";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const WSOL_MINT = new PublicKey("So11111111111111111111111111111111111111112");
const FEE_RECIPIENT = new PublicKey("BdXd6EzjCFhLmMDF1D2vm2zDrPuCzfHxyAezvPMudaU8");
const JUPITER_PROGRAM = new PublicKey("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");

// Minimum protocol fee enforced by the contract (MIN_FEE_LAMPORTS).
const MIN_FEE_LAMPORTS = 5_000_000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function airdrop(
  connection: anchor.web3.Connection,
  pubkey: PublicKey,
  sol: number
): Promise<void> {
  const sig = await connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(sig, "confirmed");
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("Balanced Vault Retrieval Flow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.sentelContract as Program<SentelContract>;

  let creator: Keypair;
  let owner1: Keypair;
  let owner2: Keypair;
  let recipient: Keypair;

  let tokenMint: PublicKey;
  let vaultId: BN;
  let balancedVaultPda: PublicKey;

  // WSOL token account owned by the vault PDA. Created once and reused.
  let vaultWsolAccount: PublicKey;

  // Nonce captured at propose-time and shared across approve/execute tests.
  let retrievalNonce: BN;

  before(async () => {
    creator   = Keypair.generate();
    owner1    = Keypair.generate();
    owner2    = Keypair.generate();
    recipient = Keypair.generate();

    for (const [kp, sol] of [
      [creator,   5] as const,
      [owner1,    2] as const,
      [owner2,    2] as const,
      [recipient, 2] as const,
    ]) {
      await airdrop(provider.connection, kp.publicKey, sol);
    }

    // The hardcoded fee_recipient must exist on localnet so the lamport
    // transfer inside execute_retrieve_transaction does not fail.
    await airdrop(provider.connection, FEE_RECIPIENT, 1);

    tokenMint = await createMint(
      provider.connection,
      creator,
      creator.publicKey,
      null,
      6
    );
  });

  // -------------------------------------------------------------------------
  // Vault creation
  // -------------------------------------------------------------------------

  it("creates a balanced vault with 2-of-2 multisig", async () => {
    vaultId = new BN(Date.now());

    const [pda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("balanced_vault"),
        creator.publicKey.toBuffer(),
        vaultId.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );
    balancedVaultPda = pda;

    const allocations = [
      { mint: tokenMint, percentage: 5000 },
      { mint: WSOL_MINT,  percentage: 5000 },
    ];

    await program.methods
      .openBalancedVault(
        vaultId,
        [owner1.publicKey, owner2.publicKey],
        2,
        allocations,
        "Test Retrieval Vault"
      )
      .accountsPartial({
        balancedVault: balancedVaultPda,
        creator: creator.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    const vault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(vault.owners).to.have.lengthOf(2);
    expect(vault.threshold).to.equal(2);
    expect(vault.isActive).to.be.true;
    expect(vault.nonce.toNumber()).to.equal(0);
    expect(vault.pendingTransactions).to.have.lengthOf(0);
  });

  // -------------------------------------------------------------------------
  // Fund vault with WSOL
  // -------------------------------------------------------------------------

  it("funds the vault with WSOL so execute_retrieve_transaction can unwrap it", async () => {
    const wsolAmount = 100_000_000; // 0.1 SOL worth of WSOL

    // Derive the vault's WSOL token account as a plain keypair (not ATA) so
    // we can create it with a known address without needing the ATA program.
    const wsolKeypair = Keypair.generate();
    vaultWsolAccount  = wsolKeypair.publicKey;

    const rentExemption = await provider.connection.getMinimumBalanceForRentExemption(165);

    const tx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: creator.publicKey,
        newAccountPubkey: wsolKeypair.publicKey,
        // lamports includes both rent and the SOL that becomes WSOL balance.
        lamports: rentExemption + wsolAmount,
        space: 165,
        programId: TOKEN_PROGRAM_ID,
      }),
      createInitializeAccountInstruction(
        wsolKeypair.publicKey,
        WSOL_MINT,
        balancedVaultPda,
        TOKEN_PROGRAM_ID
      )
    );

    await provider.sendAndConfirm(tx, [creator, wsolKeypair]);

    const info = await provider.connection.getAccountInfo(vaultWsolAccount);
    expect(info).to.not.be.null;
    // lamports = rent + wrapped SOL amount
    expect(info!.lamports).to.be.greaterThanOrEqual(wsolAmount);
  });

  // -------------------------------------------------------------------------
  // propose_retrieve_transaction
  // -------------------------------------------------------------------------

  it("proposes a retrieval transaction (no auto-approval)", async () => {
    const vault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    retrievalNonce = vault.nonce;

    const [retrieveTransactionPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("retrieve_transaction"),
        creator.publicKey.toBuffer(),
        vaultId.toArrayLike(Buffer, "le", 8),
        retrievalNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    await program.methods
      .proposeRetrieveTransaction(vaultId, retrievalNonce, recipient.publicKey)
      .accountsPartial({
        balancedVault: balancedVaultPda,
        retrieveTransaction: retrieveTransactionPda,
        creator: creator.publicKey,
        proposer: owner1.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([owner1])
      .rpc();

    const tx = await program.account.retrieveTransactionState.fetch(retrieveTransactionPda);
    expect(tx.id.toNumber()).to.equal(retrievalNonce.toNumber());
    expect(tx.recipient.toBase58()).to.equal(recipient.publicKey.toBase58());
    expect(tx.approvals).to.have.lengthOf(0); // no auto-approve
    expect(tx.executed).to.be.false;

    const updatedVault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(updatedVault.nonce.toNumber()).to.equal(retrievalNonce.toNumber() + 1);
    expect(updatedVault.pendingTransactions).to.have.lengthOf(1);
    expect(updatedVault.pendingTransactions[0].toNumber()).to.equal(retrievalNonce.toNumber());
  });

  // -------------------------------------------------------------------------
  // approve_retrieve_transaction
  // -------------------------------------------------------------------------

  it("both owners approve the retrieval to reach threshold (2 of 2)", async () => {
    const [retrieveTransactionPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("retrieve_transaction"),
        creator.publicKey.toBuffer(),
        vaultId.toArrayLike(Buffer, "le", 8),
        retrievalNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    await program.methods
      .approveRetrieveTransaction(vaultId, retrievalNonce)
      .accountsPartial({
        balancedVault: balancedVaultPda,
        retrieveTransaction: retrieveTransactionPda,
        creator: creator.publicKey,
        approver: owner1.publicKey,
      })
      .signers([owner1])
      .rpc();

    await program.methods
      .approveRetrieveTransaction(vaultId, retrievalNonce)
      .accountsPartial({
        balancedVault: balancedVaultPda,
        retrieveTransaction: retrieveTransactionPda,
        creator: creator.publicKey,
        approver: owner2.publicKey,
      })
      .signers([owner2])
      .rpc();

    const tx = await program.account.retrieveTransactionState.fetch(retrieveTransactionPda);
    expect(tx.approvals).to.have.lengthOf(2);
  });

  // -------------------------------------------------------------------------
  // execute_retrieve_transaction
  // -------------------------------------------------------------------------

  it("executes the retrieval (unwraps WSOL to SOL, closes PDA)", async () => {
    const [retrieveTransactionPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("retrieve_transaction"),
        creator.publicKey.toBuffer(),
        vaultId.toArrayLike(Buffer, "le", 8),
        retrievalNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    const wsolLamportsBefore     = await provider.connection.getBalance(vaultWsolAccount);
    const recipientBalanceBefore = await provider.connection.getBalance(recipient.publicKey);

    expect(wsolLamportsBefore).to.be.greaterThan(0, "vault WSOL account must be funded");

    // No Jupiter swaps - empty swap data and empty account counts.
    await program.methods
      .executeRetrieveTransaction(
        vaultId,
        retrievalNonce,
        [], // jupiterSwapData - none needed when vault holds only WSOL
        []  // swapAccountCounts - none
      )
      .accountsPartial({
        balancedVault: balancedVaultPda,
        retrieveTransaction: retrieveTransactionPda,
        vaultWsolAccount: vaultWsolAccount,
        recipient: recipient.publicKey,
        feeRecipient: FEE_RECIPIENT,
        creator: creator.publicKey,
        executor: owner1.publicKey,
        jupiterProgram: JUPITER_PROGRAM,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([owner1])
      .rpc();

    const recipientBalanceAfter = await provider.connection.getBalance(recipient.publicKey);
    const solReceived = recipientBalanceAfter - recipientBalanceBefore;

    // Recipient must have received SOL (WSOL unwrapped minus fee).
    expect(solReceived).to.be.greaterThan(0);
    // The fee must have been deducted - received amount < original WSOL minus rent.
    expect(solReceived).to.be.lessThan(wsolLamportsBefore);

    // retrieve_transaction PDA is closed on execution (close = executor).
    try {
      await program.account.retrieveTransactionState.fetch(retrieveTransactionPda);
      expect.fail("retrieve_transaction PDA should have been closed after execution");
    } catch (err: any) {
      expect(err.toString()).to.include("Account does not exist");
    }

    const updatedVault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(updatedVault.pendingTransactions).to.have.lengthOf(0);
  });

  // -------------------------------------------------------------------------
  // Nonce collision guard
  // -------------------------------------------------------------------------

  it("a new proposal after execution uses the next nonce with no collision", async () => {
    const vault      = await program.account.balancedVaultState.fetch(balancedVaultPda);
    const nextNonce  = vault.nonce;

    // Nonce must have advanced past the executed one.
    expect(nextNonce.toNumber()).to.be.greaterThan(retrievalNonce.toNumber());

    const [newPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("retrieve_transaction"),
        creator.publicKey.toBuffer(),
        vaultId.toArrayLike(Buffer, "le", 8),
        nextNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    // The new PDA must not exist yet.
    expect(await provider.connection.getAccountInfo(newPda)).to.be.null;

    await program.methods
      .proposeRetrieveTransaction(vaultId, nextNonce, recipient.publicKey)
      .accountsPartial({
        balancedVault: balancedVaultPda,
        retrieveTransaction: newPda,
        creator: creator.publicKey,
        proposer: owner1.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([owner1])
      .rpc();

    const newTx = await program.account.retrieveTransactionState.fetch(newPda);
    expect(newTx.executed).to.be.false;
    expect(newTx.id.toNumber()).to.equal(nextNonce.toNumber());
  });

  // -------------------------------------------------------------------------
  // close_zombie_retrieve guards
  // -------------------------------------------------------------------------

  it("close_zombie_retrieve rejects an active (non-executed, non-expired) PDA", async () => {
    const vault     = await program.account.balancedVaultState.fetch(balancedVaultPda);
    const liveNonce = new BN(vault.nonce.toNumber() - 1);

    const [livePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("retrieve_transaction"),
        creator.publicKey.toBuffer(),
        vaultId.toArrayLike(Buffer, "le", 8),
        liveNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    let threw = false;
    try {
      await (program.methods as any)
        .closeZombieRetrieve(vaultId, liveNonce)
        .accounts({
          balancedVault: balancedVaultPda,
          retrieveTransaction: livePda,
          creator: creator.publicKey,
          rentReceiver: owner1.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();
    } catch (err: any) {
      threw = true;
      expect(err.toString()).to.include("TransactionNotExpired");
    }
    expect(threw, "Expected close_zombie_retrieve to throw for active PDA").to.be.true;
  });

  it("close_zombie_retrieve rejects a caller who is not a vault owner", async () => {
    const vault     = await program.account.balancedVaultState.fetch(balancedVaultPda);
    const liveNonce = new BN(vault.nonce.toNumber() - 1);

    const [livePda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("retrieve_transaction"),
        creator.publicKey.toBuffer(),
        vaultId.toArrayLike(Buffer, "le", 8),
        liveNonce.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    // recipient is not a vault owner.
    let threw = false;
    try {
      await (program.methods as any)
        .closeZombieRetrieve(vaultId, liveNonce)
        .accounts({
          balancedVault: balancedVaultPda,
          retrieveTransaction: livePda,
          creator: creator.publicKey,
          rentReceiver: recipient.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([recipient])
        .rpc();
    } catch (err: any) {
      threw = true;
      expect(err.toString()).to.include("Unauthorized");
    }
    expect(threw, "Expected close_zombie_retrieve to throw for non-owner").to.be.true;
  });
});
