/**
 * Regression tests for multisig bypasses.
 *
 * Each test here corresponds to a way a single owner could previously act outside
 * what the other owners approved. They are kept apart from the feature suites so a
 * failure points straight at the invariant that broke.
 */

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { SentelContract } from "../target/types/sentel_contract";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { EMPTY_COMMITMENT } from "../helpers/commitment";

const FEE_RECIPIENT = new PublicKey("BdXd6EzjCFhLmMDF1D2vm2zDrPuCzfHxyAezvPMudaU8");
const JUPITER_PROGRAM = new PublicKey("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");

function errStr(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

describe("regressions: multisig bypasses", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SentelContract as Program<SentelContract>;
  const creator = provider.wallet.publicKey;

  function vaultPda(vaultId: number): PublicKey {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), creator.toBuffer(), new BN(vaultId).toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];
  }

  function txPda(vault: PublicKey, nonce: number): PublicKey {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("transaction"), vault.toBuffer(), new BN(nonce).toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];
  }

  function swapPda(vault: PublicKey, nonce: number): PublicKey {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("swap"), vault.toBuffer(), new BN(nonce).toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];
  }

  async function fund(pubkey: PublicKey, sol: number) {
    const sig = await provider.connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig, "confirmed");
  }

  it("a proposal cancelled by threshold vote can no longer be executed", async () => {
    const vaultId = 90_001;
    const vault = vaultPda(vaultId);
    const target = Keypair.generate();

    await program.methods
      .createVault([creator], 1, new BN(vaultId), "Regression A")
      .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();
    await fund(vault, 2);

    const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
    const tx = txPda(vault, nonce);

    await program.methods
      .proposeTransaction(
        target.publicKey, false, anchor.web3.SystemProgram.programId,
        new BN(0.1 * LAMPORTS_PER_SOL), new BN(vaultId), creator
      )
      .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();

    await program.methods
      .approveTransaction(creator, new BN(vaultId), new BN(nonce))
      .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();

    // Threshold is 1, so this single vote cancels the proposal outright.
    await program.methods
      .cancelTransaction(creator, new BN(vaultId), new BN(nonce))
      .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();

    const vaultState = await program.account.vaultState.fetch(vault);
    expect(vaultState.pendingTransactions.map((n) => n.toNumber())).to.not.include(nonce);

    // The proposal still holds enough approvals to execute, so only the pending-list
    // check stands between a cancelled proposal and a transfer.
    try {
      await program.methods
        .executeTransaction(creator, new BN(vaultId), new BN(nonce))
        .accountsPartial({
          vault, transaction: tx, signer: creator,
          feeRecipient: FEE_RECIPIENT, target: target.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          vaultTokenAccount: vault, targetTokenAccount: target.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      expect.fail("a cancelled proposal was executed");
    } catch (e) {
      expect(errStr(e)).to.include("ProposalCancelled");
    }
  });

  it("a token transfer cannot be redirected away from the approved token account", async () => {
    const vaultId = 90_002;
    const vault = vaultPda(vaultId);
    const target = Keypair.generate();
    const attacker = Keypair.generate();

    await program.methods
      .createVault([creator], 1, new BN(vaultId), "Regression B")
      .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();
    await fund(vault, 2);

    const payer = (provider.wallet as anchor.Wallet).payer;
    const mint = await createMint(provider.connection, payer, creator, null, 6);
    const vaultAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, mint, vault, true);
    const targetAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, mint, target.publicKey);
    const attackerAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, mint, attacker.publicKey);
    await mintTo(provider.connection, payer, mint, vaultAta.address, creator, 1_000_000);

    const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
    const tx = txPda(vault, nonce);

    await program.methods
      .proposeTransaction(targetAta.address, true, mint, new BN(500_000), new BN(vaultId), creator)
      .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
      .remainingAccounts([{ pubkey: vaultAta.address, isSigner: false, isWritable: false }])
      .rpc();

    await program.methods
      .approveTransaction(creator, new BN(vaultId), new BN(nonce))
      .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();

    // Same mint, same amount, approved threshold. Only the destination differs.
    try {
      await program.methods
        .executeTransaction(creator, new BN(vaultId), new BN(nonce))
        .accountsPartial({
          vault, transaction: tx, signer: creator,
          feeRecipient: FEE_RECIPIENT, target: targetAta.address,
          systemProgram: anchor.web3.SystemProgram.programId,
          vaultTokenAccount: vaultAta.address,
          targetTokenAccount: attackerAta.address,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
      expect.fail("tokens were sent to an account the target does not own");
    } catch (e) {
      expect(errStr(e)).to.include("InvalidTarget");
    }

    // The approved destination still works.
    await program.methods
      .executeTransaction(creator, new BN(vaultId), new BN(nonce))
      .accountsPartial({
        vault, transaction: tx, signer: creator,
        feeRecipient: FEE_RECIPIENT, target: targetAta.address,
        systemProgram: anchor.web3.SystemProgram.programId,
        vaultTokenAccount: vaultAta.address,
        targetTokenAccount: targetAta.address,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
  });

  it("a swap payload that differs from the approved commitment is rejected", async () => {
    const vaultId = 90_003;
    const vault = vaultPda(vaultId);

    await program.methods
      .createVault([creator], 1, new BN(vaultId), "Regression C")
      .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();
    await fund(vault, 2);

    const payer = (provider.wallet as anchor.Wallet).payer;
    const inputMint = await createMint(provider.connection, payer, creator, null, 6);
    const outputMint = await createMint(provider.connection, payer, creator, null, 6);
    const vaultInput = await getOrCreateAssociatedTokenAccount(provider.connection, payer, inputMint, vault, true);
    const vaultOutput = await getOrCreateAssociatedTokenAccount(provider.connection, payer, outputMint, vault, true);
    await mintTo(provider.connection, payer, inputMint, vaultInput.address, creator, 1_000_000);

    const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
    const swap = swapPda(vault, nonce);

    // Approve a commitment of all zeros, which no real payload hashes to.
    await program.methods
      .proposeSwap(inputMint, outputMint, new BN(500_000), new BN(1), EMPTY_COMMITMENT, new BN(vaultId), creator)
      .accountsPartial({ vault, swapTransaction: swap, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
      .remainingAccounts([{ pubkey: vaultInput.address, isSigner: false, isWritable: false }])
      .rpc();

    await program.methods
      .approveSwap(creator, new BN(vaultId), new BN(nonce))
      .accountsPartial({ vault, swapTransaction: swap, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
      .rpc();

    try {
      await program.methods
        .executeSwap(creator, new BN(vaultId), new BN(nonce), Buffer.from([0xde, 0xad, 0xbe, 0xef]))
        .accountsPartial({
          vault, swapTransaction: swap,
          vaultInputTokenAccount: vaultInput.address,
          vaultOutputTokenAccount: vaultOutput.address,
          signer: creator, feeRecipient: FEE_RECIPIENT,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: anchor.web3.SystemProgram.programId,
          jupiterProgram: JUPITER_PROGRAM,
        })
        .rpc();
      expect.fail("an unapproved Jupiter payload was executed");
    } catch (e) {
      expect(errStr(e)).to.include("SwapPayloadMismatch");
    }
  });
});
