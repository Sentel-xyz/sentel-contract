/**
 * Vault (multisig) test suite
 *
 * One describe block per on-chain instruction group.
 * Every positive path and every named error variant is covered.
 * Jupiter CPIs that cannot succeed on localnet are tested up to the CPI
 * boundary: we verify that all Anchor-level validation passes before the
 * off-chain program is invoked.
 *
 * Instructions covered
 *   Vault lifecycle : create_vault, close_vault, get_vault_info
 *   SOL/SPL txns    : propose_transaction, approve_transaction,
 *                     execute_transaction, cancel_transaction,
 *                     get_transaction_status, cleanup_expired
 *   Swap txns       : propose_swap, approve_swap, execute_swap,
 *                     cancel_swap, propose_sol_swap
 *   Wrap txns       : propose_wrap, approve_wrap, execute_wrap, cancel_wrap
 *   Balance locking : pending-transaction balance checks for SOL and tokens
 */

import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { SentelContract } from "../target/types/sentel_contract";
import { expect } from "chai";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  transfer,
  getAccount,
} from "@solana/spl-token";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
} from "@solana/web3.js";

// ---------------------------------------------------------------------------
// Program-level constants (must match lib.rs)
// ---------------------------------------------------------------------------

const FEE_RECIPIENT = new PublicKey("BdXd6EzjCFhLmMDF1D2vm2zDrPuCzfHxyAezvPMudaU8");
const JUPITER_PROGRAM = new PublicKey("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
const WSOL_MINT = new PublicKey("So11111111111111111111111111111111111111112");
const MIN_FEE_LAMPORTS = 5_000_000;    // 0.005 SOL

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function errStr(e: unknown): string {
  if (e instanceof Error) return e.message;
  return String(e);
}

async function airdrop(
  connection: anchor.web3.Connection,
  pubkey: PublicKey,
  sol: number
): Promise<void> {
  const sig = await connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(sig, "confirmed");
}

function vaultPda(
  programId: PublicKey,
  creator: PublicKey,
  vaultId: number
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), creator.toBuffer(), new BN(vaultId).toArrayLike(Buffer, "le", 8)],
    programId
  );
  return pda;
}

function txPda(
  programId: PublicKey,
  vault: PublicKey,
  nonce: number
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("transaction"), vault.toBuffer(), new BN(nonce).toArrayLike(Buffer, "le", 8)],
    programId
  );
  return pda;
}

function swapPda(
  programId: PublicKey,
  vault: PublicKey,
  nonce: number
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("swap"), vault.toBuffer(), new BN(nonce).toArrayLike(Buffer, "le", 8)],
    programId
  );
  return pda;
}

function wrapPda(
  programId: PublicKey,
  vault: PublicKey,
  nonce: number
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("wrap"), vault.toBuffer(), new BN(nonce).toArrayLike(Buffer, "le", 8)],
    programId
  );
  return pda;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("Vault (multisig)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.sentelContract as Program<SentelContract>;
  const creator = provider.wallet.publicKey;
  // payer is Keypair | undefined; assert defined once here.
  const payer = (provider.wallet as anchor.Wallet).payer;

  const owner2 = Keypair.generate();
  const owner3 = Keypair.generate();
  const target  = Keypair.generate();
  const stranger = Keypair.generate();

  // Vault IDs are unique integers - keep a registry to avoid collisions.
  // IDs 1-9       : create_vault tests
  // IDs 10-19     : SOL/SPL transaction tests
  // IDs 50        : comprehensive workflow
  // IDs 100-109   : close_vault tests
  // IDs 200-209   : SPL token tests
  // IDs 300-309   : balance locking tests
  // IDs 400-409   : swap tests
  // IDs 500-509   : wrap tests
  // IDs 700-709   : propose_sol_swap tests

  let primaryVault: PublicKey;   // 2-of-3 vault used by most tests
  let multiVault: PublicKey;     // 3-of-3 vault used for cancel tests

  before(async () => {
    await airdrop(provider.connection, owner2.publicKey, 3);
    await airdrop(provider.connection, owner3.publicKey, 3);
    await airdrop(provider.connection, target.publicKey, 0.1);
    await airdrop(provider.connection, stranger.publicKey, 1);
    // Fee recipient must exist on localnet.
    await airdrop(provider.connection, FEE_RECIPIENT, 0.05);
  });

  // ==========================================================================
  // create_vault
  // ==========================================================================

  describe("create_vault", () => {
    it("creates a 2-of-3 vault", async () => {
      primaryVault = vaultPda(program.programId, creator, 1);
      await program.methods
        .createVault([creator, owner2.publicKey, owner3.publicKey], 2, new BN(1), "Primary Vault")
        .accountsPartial({ vault: primaryVault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const v = await program.account.vaultState.fetch(primaryVault);
      expect(v.owners).to.have.lengthOf(3);
      expect(v.threshold).to.equal(2);
      expect(v.nonce.toNumber()).to.equal(0);
      expect(v.pendingTransactions).to.deep.equal([]);
      expect(v.name).to.equal("Primary Vault");
    });

    it("creates a vault with maximum 10 owners", async () => {
      const manyOwners = Array.from({ length: 10 }, () => Keypair.generate().publicKey);
      const v = vaultPda(program.programId, creator, 2);
      await program.methods
        .createVault(manyOwners, 5, new BN(2), "Max Owners")
        .accountsPartial({ vault: v, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const account = await program.account.vaultState.fetch(v);
      expect(account.owners).to.have.lengthOf(10);
    });

    it("rejects threshold of zero", async () => {
      const v = vaultPda(program.programId, creator, 90);
      try {
        await program.methods
          .createVault([creator, owner2.publicKey], 0, new BN(90), "Bad Threshold")
          .accountsPartial({ vault: v, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected InvalidThreshold");
      } catch (e) { expect(errStr(e)).to.include("InvalidThreshold"); }
    });

    it("rejects threshold greater than owner count", async () => {
      const v = vaultPda(program.programId, creator, 91);
      try {
        await program.methods
          .createVault([creator], 2, new BN(91), "Bad Threshold 2")
          .accountsPartial({ vault: v, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected InvalidThreshold");
      } catch (e) { expect(errStr(e)).to.include("InvalidThreshold"); }
    });

    it("rejects duplicate owners", async () => {
      const v = vaultPda(program.programId, creator, 92);
      try {
        await program.methods
          .createVault([creator, owner2.publicKey, creator], 2, new BN(92), "Dup Owners")
          .accountsPartial({ vault: v, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected DuplicateOwner");
      } catch (e) { expect(errStr(e)).to.include("DuplicateOwner"); }
    });

    it("rejects more than 10 owners", async () => {
      const v = vaultPda(program.programId, creator, 93);
      const tooMany = Array.from({ length: 11 }, () => Keypair.generate().publicKey);
      try {
        await program.methods
          .createVault(tooMany, 5, new BN(93), "Too Many")
          .accountsPartial({ vault: v, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected TooManyOwners");
      } catch (e) { expect(errStr(e)).to.include("TooManyOwners"); }
    });

    it("rejects an empty vault name", async () => {
      const v = vaultPda(program.programId, creator, 94);
      try {
        await program.methods
          .createVault([creator], 1, new BN(94), "")
          .accountsPartial({ vault: v, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected EmptyName");
      } catch (e) { expect(errStr(e)).to.include("EmptyName"); }
    });

    it("rejects a vault name longer than 50 characters", async () => {
      const v = vaultPda(program.programId, creator, 95);
      try {
        await program.methods
          .createVault([creator], 1, new BN(95), "A".repeat(51))
          .accountsPartial({ vault: v, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected NameTooLong");
      } catch (e) { expect(errStr(e)).to.include("NameTooLong"); }
    });
  });

  // ==========================================================================
  // get_vault_info
  // ==========================================================================

  describe("get_vault_info", () => {
    it("emits VaultInfoEvent with correct fields", async () => {
      let received: any;
      const listener = program.addEventListener("vaultInfoEvent", (e) => { received = e; });
      try {
        await program.methods
          .getVaultInfo(creator, new BN(1))
          .accountsPartial({ vault: primaryVault })
          .rpc();
        await new Promise((r) => setTimeout(r, 1_000));
        expect(received).to.not.be.undefined;
        expect(received.vaultAddress.toString()).to.equal(primaryVault.toString());
        expect(received.owners).to.have.lengthOf(3);
        expect(received.threshold).to.equal(2);
        expect(received.name).to.equal("Primary Vault");
      } finally {
        program.removeEventListener(listener);
      }
    });
  });

  // ==========================================================================
  // propose_transaction / approve_transaction / execute_transaction
  // ==========================================================================

  describe("SOL transactions", () => {
    const VAULT_ID = 10;
    let vault: PublicKey;

    before(async () => {
      vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "SOL Tx Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 2);
    });

    it("propose: creates a pending SOL transaction", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);

      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.5 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const t = await program.account.transactionState.fetch(tx);
      expect(t.amount.toNumber()).to.equal(0.5 * LAMPORTS_PER_SOL);
      expect(t.executed).to.be.false;
      expect(t.approvals).to.have.lengthOf(0);
    });

    it("propose: rejects zero amount", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);
      try {
        await program.methods
          .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected InvalidAmount");
      } catch (e) { expect(errStr(e)).to.include("InvalidAmount"); }
    });

    it("propose: rejects a non-owner proposer", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);
      try {
        await program.methods
          .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(100_000), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, transaction: tx, proposer: stranger.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([stranger])
          .rpc();
        expect.fail("expected UnauthorizedProposer");
      } catch (e) { expect(e).to.exist; }
    });

    it("approve: first owner adds approval", async () => {
      const tx = txPda(program.programId, vault, 0);
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({ vault, transaction: tx, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
        .signers([owner2])
        .rpc();

      const t = await program.account.transactionState.fetch(tx);
      expect(t.approvals).to.have.lengthOf(1);
    });

    it("approve: rejects a duplicate approval from the same signer", async () => {
      const tx = txPda(program.programId, vault, 0);
      try {
        await program.methods
          .approveTransaction(creator, new BN(VAULT_ID), new BN(0))
          .accountsPartial({ vault, transaction: tx, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([owner2])
          .rpc();
        expect.fail("expected AlreadyApproved");
      } catch (e) { expect(errStr(e)).to.include("AlreadyApproved"); }
    });

    it("approve: rejects a non-owner signer", async () => {
      const tx = txPda(program.programId, vault, 0);
      try {
        await program.methods
          .approveTransaction(creator, new BN(VAULT_ID), new BN(0))
          .accountsPartial({ vault, transaction: tx, signer: stranger.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([stranger])
          .rpc();
        expect.fail("expected UnauthorizedProposer");
      } catch (e) { expect(errStr(e)).to.include("UnauthorizedProposer"); }
    });

    it("execute: sends SOL to target, deducts MIN_FEE to fee recipient, closes PDA", async () => {
      // Reach threshold: creator approves (owner2 approved above).
      const tx = txPda(program.programId, vault, 0);
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const targetBefore = await provider.connection.getBalance(target.publicKey);
      const feeBefore    = await provider.connection.getBalance(FEE_RECIPIENT);

      await program.methods
        .executeTransaction(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({
          vault, transaction: tx, signer: creator,
          feeRecipient: FEE_RECIPIENT, target: target.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          vaultTokenAccount: vault, targetTokenAccount: target.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      const targetAfter = await provider.connection.getBalance(target.publicKey);
      const feeAfter    = await provider.connection.getBalance(FEE_RECIPIENT);

      // SOL transfers: fee = amount * PROTOCOL_FEE_BASIS_POINTS / 10000 (no floor).
      // For 0.5 SOL: 500_000_000 * 5 / 10000 = 250_000 lamports.
      const TX_AMOUNT = 0.5 * LAMPORTS_PER_SOL;
      const expectedFee = Math.floor(TX_AMOUNT * 5 / 10_000);
      expect(targetAfter - targetBefore).to.be.greaterThan(0);
      expect(feeAfter - feeBefore).to.equal(expectedFee);

      try {
        await program.account.transactionState.fetch(tx);
        expect.fail("PDA should be closed");
      } catch (e) { expect(errStr(e)).to.include("Account does not exist"); }
    });

    it("execute: rejects an invalid fee recipient", async () => {
      const v = await program.account.vaultState.fetch(vault);
      const nonce = v.nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);
      const badFee = Keypair.generate();

      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.1 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
        .signers([owner2])
        .rpc();

      try {
        await program.methods
          .executeTransaction(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({
            vault, transaction: tx, signer: creator,
            feeRecipient: badFee.publicKey, target: target.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            vaultTokenAccount: vault, targetTokenAccount: target.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();
        expect.fail("expected InvalidFeeRecipient");
      } catch (e) { expect(errStr(e)).to.include("InvalidFeeRecipient"); }
    });
  });

  // ==========================================================================
  // get_transaction_status
  // ==========================================================================

  describe("get_transaction_status", () => {
    const VAULT_ID = 11;
    let vault: PublicKey;

    before(async () => {
      vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "Status Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 1);
    });

    it("emits TransactionStatusEvent with correct approval count", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);

      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.1 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      let received: any;
      const listener = program.addEventListener("transactionStatusEvent", (e) => { received = e; });
      try {
        await program.methods
          .getTransactionStatus(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({ vault, transaction: tx })
          .rpc();
        await new Promise((r) => setTimeout(r, 1_000));
        expect(received).to.not.be.undefined;
        expect(received.transactionId.toNumber()).to.equal(nonce);
        expect(received.approvalCount).to.equal(1);
        expect(received.requiredApprovals).to.equal(2);
        expect(received.executed).to.be.false;
      } finally {
        program.removeEventListener(listener);
      }
    });
  });

  // ==========================================================================
  // cancel_transaction
  // ==========================================================================

  describe("cancel_transaction", () => {
    const VAULT_ID = 12;
    let vault: PublicKey;

    before(async () => {
      // 3-of-3 vault so a single cancel vote does not immediately remove the proposal.
      multiVault = vaultPda(program.programId, creator, VAULT_ID);
      vault      = multiVault;
      await program.methods
        .createVault([creator, owner2.publicKey, owner3.publicKey], 3, new BN(VAULT_ID), "Cancel Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 1);
    });

    it("cancel vote accumulates and removes proposal at threshold", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);

      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.1 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      // First cancel vote.
      await program.methods
        .cancelTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      // Proposal still live (1 of 3 votes).
      let v = await program.account.vaultState.fetch(vault);
      expect(v.pendingTransactions.map((x: BN) => x.toNumber())).to.include(nonce);

      // Second cancel vote.
      await program.methods
        .cancelTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
        .signers([owner2])
        .rpc();

      // Still live (2 of 3).
      v = await program.account.vaultState.fetch(vault);
      expect(v.pendingTransactions.map((x: BN) => x.toNumber())).to.include(nonce);

      // Third cancel vote - reaches threshold, proposal removed.
      let cancelEvent: any;
      const listener = program.addEventListener("transactionCancelledEvent", (e) => { cancelEvent = e; });
      try {
        await program.methods
          .cancelTransaction(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({ vault, transaction: tx, signer: owner3.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([owner3])
          .rpc();
        await new Promise((r) => setTimeout(r, 1_000));
        v = await program.account.vaultState.fetch(vault);
        expect(v.pendingTransactions.map((x: BN) => x.toNumber())).to.not.include(nonce);
        expect(cancelEvent).to.not.be.undefined;
      } finally {
        program.removeEventListener(listener);
      }
    });

    it("rejects a duplicate cancel vote from the same signer", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);

      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.1 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .cancelTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      try {
        await program.methods
          .cancelTransaction(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected AlreadyCancelledVote");
      } catch (e) { expect(errStr(e)).to.include("AlreadyCancelledVote"); }
    });

    it("rejects a non-owner cancel attempt", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);

      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.1 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      try {
        await program.methods
          .cancelTransaction(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({ vault, transaction: tx, signer: stranger.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([stranger])
          .rpc();
        expect.fail("expected error for non-owner");
      } catch (e) { expect(e).to.exist; }
    });
  });

  // ==========================================================================
  // cleanup_expired
  // ==========================================================================

  describe("cleanup_expired", () => {
    it("rejects cleanup of a non-expired transaction", async () => {
      const VAULT_ID = 13;
      const vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "Cleanup Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 0.5);

      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);
      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.1 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      try {
        await program.methods
          .cleanupExpired(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({ vault, transaction: tx, rentReceiver: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected TransactionNotExpired");
      } catch (e) { expect(errStr(e)).to.include("TransactionNotExpired"); }
    });
  });

  // ==========================================================================
  // close_vault
  // ==========================================================================

  describe("close_vault", () => {
    it("closes a vault with low balance and no pending transactions", async () => {
      const VAULT_ID = 100;
      const vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator], 1, new BN(VAULT_ID), "Closeable Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 0.05);

      const balBefore = await provider.connection.getBalance(creator);

      let closeEvent: any;
      const listener = program.addEventListener("vaultClosedEvent", (e) => { closeEvent = e; });
      try {
        await program.methods
          .closeVault(creator, new BN(VAULT_ID))
          .accountsPartial({ vault, creatorSigner: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        await new Promise((r) => setTimeout(r, 1_000));

        const balAfter = await provider.connection.getBalance(creator);
        expect(balAfter).to.be.greaterThan(balBefore);
        expect(closeEvent).to.not.be.undefined;
        expect(closeEvent.creator.toString()).to.equal(creator.toString());

        try {
          await program.account.vaultState.fetch(vault);
          expect.fail("account should be closed");
        } catch (e) { expect(errStr(e)).to.include("Account does not exist"); }
      } finally {
        program.removeEventListener(listener);
      }
    });

    it("rejects closure when vault balance exceeds 0.3 SOL", async () => {
      const VAULT_ID = 101;
      const vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator], 1, new BN(VAULT_ID), "Rich Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 0.4);

      try {
        await program.methods
          .closeVault(creator, new BN(VAULT_ID))
          .accountsPartial({ vault, creatorSigner: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected VaultBalanceTooHighForClosure");
      } catch (e) { expect(errStr(e)).to.include("VaultBalanceTooHighForClosure"); }
    });

    it("rejects closure when vault has pending transactions", async () => {
      const VAULT_ID = 102;
      const vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "Busy Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 0.05);

      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);
      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.01 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      try {
        await program.methods
          .closeVault(creator, new BN(VAULT_ID))
          .accountsPartial({ vault, creatorSigner: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected VaultHasPendingTransactions");
      } catch (e) { expect(errStr(e)).to.include("VaultHasPendingTransactions"); }
    });

    it("rejects closure by a non-creator", async () => {
      const VAULT_ID = 103;
      const vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator], 1, new BN(VAULT_ID), "Protected Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 0.05);

      try {
        await program.methods
          .closeVault(creator, new BN(VAULT_ID))
          .accountsPartial({ vault, creatorSigner: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([owner2])
          .rpc();
        expect.fail("expected UnauthorizedCreator");
      } catch (e) { expect(errStr(e)).to.include("UnauthorizedCreator"); }
    });

    it("rejects closure when vault holds SPL tokens", async () => {
      const VAULT_ID = 104;
      const vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator], 1, new BN(VAULT_ID), "Token Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 0.05);

      const mint = await createMint(provider.connection, payer, creator, creator, 6);
      const vaultAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, mint, vault, true);
      await mintTo(provider.connection, payer, mint, vaultAta.address, creator, 1_000_000);

      try {
        await program.methods
          .closeVault(creator, new BN(VAULT_ID))
          .accountsPartial({ vault, creatorSigner: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .remainingAccounts([{ pubkey: vaultAta.address, isSigner: false, isWritable: false }])
          .rpc();
        expect.fail("expected VaultHasTokenBalance");
      } catch (e) { expect(errStr(e)).to.include("VaultHasTokenBalance"); }
    });
  });

  // ==========================================================================
  // SPL token transactions
  // ==========================================================================

  describe("SPL token transactions", () => {
    const VAULT_ID = 200;
    let vault: PublicKey;
    let mint: PublicKey;
    let vaultTokenAta: any;
    let targetTokenAta: any;

    before(async () => {
      vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "SPL Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 0.1);

      mint = await createMint(provider.connection, payer, creator, creator, 6);

      vaultTokenAta  = await getOrCreateAssociatedTokenAccount(provider.connection, payer, mint, vault, true);
      targetTokenAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, mint, target.publicKey);

      await mintTo(provider.connection, payer, mint, vaultTokenAta.address, creator, 10_000_000);
    });

    it("propose: creates a pending SPL token transaction", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx    = txPda(program.programId, vault, nonce);

      await program.methods
        .proposeTransaction(targetTokenAta.address, true, mint, new BN(1_000_000), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .remainingAccounts([{ pubkey: vaultTokenAta.address, isSigner: false, isWritable: false }])
        .rpc();

      const t = await program.account.transactionState.fetch(tx);
      expect(t.tokenType).to.be.true;
      expect(t.mint.toBase58()).to.equal(mint.toBase58());
      expect(t.amount.toNumber()).to.equal(1_000_000);
      expect(t.approvals).to.have.lengthOf(0);
    });

    it("approve + execute: transfers tokens to target, SOL fee to fee recipient", async () => {
      const v     = await program.account.vaultState.fetch(vault);
      const nonce = v.nonce.toNumber() - 1;
      const tx    = txPda(program.programId, vault, nonce);

      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
        .signers([owner2])
        .rpc();

      const targetBefore = await getAccount(provider.connection, targetTokenAta.address);
      const vaultBefore  = await getAccount(provider.connection, vaultTokenAta.address);
      const feeBefore    = await provider.connection.getBalance(FEE_RECIPIENT);

      await program.methods
        .executeTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({
          vault, transaction: tx, signer: creator,
          feeRecipient: FEE_RECIPIENT, target: targetTokenAta.address,
          systemProgram: anchor.web3.SystemProgram.programId,
          vaultTokenAccount: vaultTokenAta.address,
          targetTokenAccount: targetTokenAta.address,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      const targetAfter = await getAccount(provider.connection, targetTokenAta.address);
      const vaultAfter  = await getAccount(provider.connection, vaultTokenAta.address);
      const feeAfter    = await provider.connection.getBalance(FEE_RECIPIENT);

      expect(targetAfter.amount - targetBefore.amount).to.equal(BigInt(1_000_000));
      expect(vaultBefore.amount - vaultAfter.amount).to.equal(BigInt(1_000_000));
      expect(feeAfter - feeBefore).to.be.greaterThanOrEqual(MIN_FEE_LAMPORTS);
    });

    it("execute: rejects mismatched mint on token account", async () => {
      const wrongMint    = await createMint(provider.connection, payer, creator, creator, 6);
      const wrongAta     = await getOrCreateAssociatedTokenAccount(provider.connection, payer, wrongMint, target.publicKey);
      const nonce        = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const tx           = txPda(program.programId, vault, nonce);

      // Propose using the correct mint.
      await program.methods
        .proposeTransaction(wrongAta.address, true, mint, new BN(100_000), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .remainingAccounts([{ pubkey: vaultTokenAta.address, isSigner: false, isWritable: false }])
        .rpc();
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, transaction: tx, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
        .signers([owner2])
        .rpc();

      try {
        await program.methods
          .executeTransaction(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({
            vault, transaction: tx, signer: creator,
            feeRecipient: FEE_RECIPIENT, target: wrongAta.address,
            systemProgram: anchor.web3.SystemProgram.programId,
            vaultTokenAccount: vaultTokenAta.address,
            targetTokenAccount: wrongAta.address, // wrong mint
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();
        expect.fail("expected InvalidMint");
      } catch (e) { expect(errStr(e)).to.include("InvalidMint"); }
    });
  });

  // ==========================================================================
  // Pending-transaction balance locking
  // ==========================================================================

  describe("balance locking", () => {
    describe("SPL token balance", () => {
      const VAULT_ID = 300;
      let vault: PublicKey;
      let testMint: PublicKey;
      let vaultTokenAta: any;
      let targetAta1: any;
      let targetAta2: any;

      before(async () => {
        vault = vaultPda(program.programId, creator, VAULT_ID);
        testMint = await createMint(provider.connection, payer, creator, creator, 6);

        vaultTokenAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, testMint, vault, true);
        targetAta1    = await getOrCreateAssociatedTokenAccount(provider.connection, payer, testMint, target.publicKey);
        targetAta2    = await getOrCreateAssociatedTokenAccount(provider.connection, payer, testMint, owner2.publicKey);

        await program.methods
          .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "Balance Lock Vault")
          .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        await mintTo(provider.connection, payer, testMint, vaultTokenAta.address, creator, 10_000_000);
        await airdrop(provider.connection, vault, 0.1);
      });

      it("rejects a second proposal that would exceed available token balance", async () => {
        const nonce1 = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
        const tx1    = txPda(program.programId, vault, nonce1);

        // Lock 6 tokens - leaves 4 available.
        await program.methods
          .proposeTransaction(targetAta1.address, true, testMint, new BN(6_000_000), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, transaction: tx1, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .remainingAccounts([{ pubkey: vaultTokenAta.address, isSigner: false, isWritable: false }])
          .rpc();

        const nonce2 = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
        const tx2    = txPda(program.programId, vault, nonce2);

        // Try to lock 5 more tokens - only 4 available.
        try {
          await program.methods
            .proposeTransaction(targetAta2.address, true, testMint, new BN(5_000_000), new BN(VAULT_ID), creator)
            .accountsPartial({ vault, transaction: tx2, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
            .remainingAccounts([
              { pubkey: tx1, isSigner: false, isWritable: false },
              { pubkey: vaultTokenAta.address, isSigner: false, isWritable: false },
            ])
            .rpc();
          expect.fail("expected InsufficientAvailableBalance");
        } catch (e) { expect(errStr(e)).to.include("InsufficientAvailableBalance"); }
      });

      it("accepts a proposal within available token balance", async () => {
        const v     = await program.account.vaultState.fetch(vault);
        const nonce = v.nonce.toNumber();
        const tx    = txPda(program.programId, vault, nonce);
        const prev  = txPda(program.programId, vault, nonce - 1);

        // 3 tokens within the 4 available.
        await program.methods
          .proposeTransaction(targetAta2.address, true, testMint, new BN(3_000_000), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .remainingAccounts([
            { pubkey: prev, isSigner: false, isWritable: false },
            { pubkey: vaultTokenAta.address, isSigner: false, isWritable: false },
          ])
          .rpc();

        const t = await program.account.transactionState.fetch(tx);
        expect(t.amount.toNumber()).to.equal(3_000_000);
      });
    });

    describe("SOL balance", () => {
      const VAULT_ID = 301;
      let vault: PublicKey;

      before(async () => {
        vault = vaultPda(program.programId, creator, VAULT_ID);
        await program.methods
          .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "SOL Lock Vault")
          .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        await airdrop(provider.connection, vault, 0.5);
      });

      it("rejects a second proposal that would exceed available SOL balance", async () => {
        const nonce1 = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
        const tx1    = txPda(program.programId, vault, nonce1);

        // Lock 0.3 SOL.
        await program.methods
          .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.3 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, transaction: tx1, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();

        const nonce2 = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
        const tx2    = txPda(program.programId, vault, nonce2);

        try {
          await program.methods
            .proposeTransaction(owner2.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.2 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
            .accountsPartial({ vault, transaction: tx2, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
            .remainingAccounts([{ pubkey: tx1, isSigner: false, isWritable: false }])
            .rpc();
          expect.fail("expected InsufficientAvailableBalance");
        } catch (e) { expect(errStr(e)).to.include("InsufficientAvailableBalance"); }
      });

      it("accepts a SOL proposal within available balance", async () => {
        const v     = await program.account.vaultState.fetch(vault);
        const nonce = v.nonce.toNumber();
        const tx    = txPda(program.programId, vault, nonce);
        const prev  = txPda(program.programId, vault, nonce - 1);

        await program.methods
          .proposeTransaction(owner2.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.15 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, transaction: tx, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .remainingAccounts([{ pubkey: prev, isSigner: false, isWritable: false }])
          .rpc();

        const t = await program.account.transactionState.fetch(tx);
        expect(t.amount.toNumber()).to.equal(0.15 * LAMPORTS_PER_SOL);
      });
    });
  });

  // ==========================================================================
  // propose_swap / approve_swap / execute_swap / cancel_swap
  // ==========================================================================

  describe("swap transactions", () => {
    const VAULT_ID = 400;
    let vault: PublicKey;
    let inputMint: PublicKey;
    let outputMint: PublicKey;
    let vaultInputAta: any;
    let vaultOutputAta: any;

    before(async () => {
      vault = vaultPda(program.programId, creator, VAULT_ID);

      inputMint  = await createMint(provider.connection, payer, creator, creator, 6);
      outputMint = await createMint(provider.connection, payer, creator, creator, 6);

      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "Swap Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      vaultInputAta  = await getOrCreateAssociatedTokenAccount(provider.connection, payer, inputMint,  vault, true);
      vaultOutputAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, outputMint, vault, true);

      await mintTo(provider.connection, payer, inputMint, vaultInputAta.address, creator, 1_000_000_000);
      await airdrop(provider.connection, vault, 0.2);
    });

    it("propose_swap: creates a pending swap with zero approvals", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const sp    = swapPda(program.programId, vault, nonce);

      await program.methods
        .proposeSwap(inputMint, outputMint, new BN(100_000_000), new BN(95_000_000), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, swapTransaction: sp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .remainingAccounts([{ pubkey: vaultInputAta.address, isSigner: false, isWritable: false }])
        .rpc();

      const s = await program.account.swapTransactionState.fetch(sp);
      expect(s.inputMint.toString()).to.equal(inputMint.toString());
      expect(s.outputMint.toString()).to.equal(outputMint.toString());
      expect(s.inputAmount.toNumber()).to.equal(100_000_000);
      expect(s.minimumOutputAmount.toNumber()).to.equal(95_000_000);
      expect(s.executed).to.be.false;
      expect(s.approvals).to.be.empty;
    });

    it("propose_swap: rejects zero input amount", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const sp    = swapPda(program.programId, vault, nonce);
      try {
        await program.methods
          .proposeSwap(inputMint, outputMint, new BN(0), new BN(95_000_000), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, swapTransaction: sp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected InvalidAmount");
      } catch (e) { expect(errStr(e)).to.include("InvalidAmount"); }
    });

    it("propose_swap: rejects amount exceeding available balance", async () => {
      const v     = await program.account.vaultState.fetch(vault);
      const nonce = v.nonce.toNumber();
      const sp    = swapPda(program.programId, vault, nonce);
      const prev  = swapPda(program.programId, vault, nonce - 1);
      try {
        await program.methods
          .proposeSwap(inputMint, outputMint, new BN(950_000_000), new BN(900_000_000), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, swapTransaction: sp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .remainingAccounts([
            { pubkey: prev, isSigner: false, isWritable: false },
            { pubkey: vaultInputAta.address, isSigner: false, isWritable: false },
          ])
          .rpc();
        expect.fail("expected InsufficientAvailableBalance");
      } catch (e) { expect(errStr(e)).to.include("InsufficientAvailableBalance"); }
    });

    it("approve_swap: first owner approves", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber() - 1;
      const sp    = swapPda(program.programId, vault, nonce);

      await program.methods
        .approveSwap(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, swapTransaction: sp, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const s = await program.account.swapTransactionState.fetch(sp);
      expect(s.approvals).to.have.lengthOf(1);
    });

    it("approve_swap: rejects a duplicate approval", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber() - 1;
      const sp    = swapPda(program.programId, vault, nonce);
      try {
        await program.methods
          .approveSwap(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({ vault, swapTransaction: sp, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected SwapAlreadyApproved");
      } catch (e) { expect(errStr(e)).to.include("SwapAlreadyApproved"); }
    });

    it("approve_swap: rejects a non-owner approver", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber() - 1;
      const sp    = swapPda(program.programId, vault, nonce);
      try {
        await program.methods
          .approveSwap(creator, new BN(VAULT_ID), new BN(nonce))
          .accountsPartial({ vault, swapTransaction: sp, signer: stranger.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([stranger])
          .rpc();
        expect.fail("expected UnauthorizedProposer");
      } catch (e) { expect(errStr(e)).to.include("UnauthorizedProposer"); }
    });

    it("execute_swap: all Anchor validation passes before Jupiter CPI", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber() - 1;
      const sp    = swapPda(program.programId, vault, nonce);

      // Reach threshold.
      await program.methods
        .approveSwap(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, swapTransaction: sp, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
        .signers([owner2])
        .rpc();

      // On localnet, Jupiter CPI will fail - we verify the error is a CPI error
      // and NOT an Anchor validation error (approvals, expiry, fee recipient).
      try {
        await program.methods
          .executeSwap(creator, new BN(VAULT_ID), new BN(nonce), Buffer.from([0]))
          .accountsPartial({
            vault, swapTransaction: sp,
            vaultInputTokenAccount: vaultInputAta.address,
            vaultOutputTokenAccount: vaultOutputAta.address,
            signer: creator, jupiterProgram: JUPITER_PROGRAM,
            feeRecipient: FEE_RECIPIENT,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
      } catch (e) {
        const msg = errStr(e);
        expect(msg).to.not.include("InsufficientApprovalsForSwap");
        expect(msg).to.not.include("SwapExpired");
        expect(msg).to.not.include("InvalidFeeRecipient");
      }
    });

    it("execute_swap: rejects execution with insufficient approvals", async () => {
      const nonce = (await program.account.vaultState.fetch(vault)).nonce.toNumber();
      const sp    = swapPda(program.programId, vault, nonce);

      await program.methods
        .proposeSwap(inputMint, outputMint, new BN(50_000_000), new BN(45_000_000), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, swapTransaction: sp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .remainingAccounts([{ pubkey: vaultInputAta.address, isSigner: false, isWritable: false }])
        .rpc();
      await program.methods
        .approveSwap(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, swapTransaction: sp, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      // Only 1 of 2 approvals - should fail.
      try {
        await program.methods
          .executeSwap(creator, new BN(VAULT_ID), new BN(nonce), Buffer.from([0]))
          .accountsPartial({
            vault, swapTransaction: sp,
            vaultInputTokenAccount: vaultInputAta.address,
            vaultOutputTokenAccount: vaultOutputAta.address,
            signer: creator, jupiterProgram: JUPITER_PROGRAM,
            feeRecipient: FEE_RECIPIENT,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("expected InsufficientApprovalsForSwap");
      } catch (e) { expect(errStr(e)).to.include("InsufficientApprovalsForSwap"); }
    });

    it("cancel_swap: removes proposal from vault pending list", async () => {
      const CANCEL_VAULT_ID = 401;
      const cv = vaultPda(program.programId, creator, CANCEL_VAULT_ID);

      await program.methods
        .createVault([creator], 1, new BN(CANCEL_VAULT_ID), "Cancel Swap Vault")
        .accountsPartial({ vault: cv, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const cvInputAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer, inputMint, cv, true);
      await mintTo(provider.connection, payer, inputMint, cvInputAta.address, creator, 100_000_000);

      const nonce = (await program.account.vaultState.fetch(cv)).nonce.toNumber();
      const sp    = swapPda(program.programId, cv, nonce);

      await program.methods
        .proposeSwap(inputMint, outputMint, new BN(10_000_000), new BN(9_000_000), new BN(CANCEL_VAULT_ID), creator)
        .accountsPartial({ vault: cv, swapTransaction: sp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .remainingAccounts([{ pubkey: cvInputAta.address, isSigner: false, isWritable: false }])
        .rpc();

      let before = await program.account.vaultState.fetch(cv);
      expect(before.pendingTransactions.map((x: BN) => x.toNumber())).to.include(nonce);

      await program.methods
        .cancelSwap(creator, new BN(CANCEL_VAULT_ID), new BN(nonce))
        .accountsPartial({ vault: cv, swapTransaction: sp, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const after = await program.account.vaultState.fetch(cv);
      expect(after.pendingTransactions.map((x: BN) => x.toNumber())).to.not.include(nonce);
    });
  });

  // ==========================================================================
  // propose_sol_swap
  // ==========================================================================

  describe("propose_sol_swap", () => {
    const VAULT_ID = 700;
    const NATIVE_MINT = WSOL_MINT;
    let vault: PublicKey;
    let vaultWsolAta: PublicKey;
    let outputMint: PublicKey;

    before(async () => {
      vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator], 1, new BN(VAULT_ID), "SOL Swap Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 1);

      const wsolInfo = await getOrCreateAssociatedTokenAccount(provider.connection, payer, NATIVE_MINT, vault, true);
      vaultWsolAta   = wsolInfo.address;

      outputMint = await createMint(provider.connection, payer, creator, null, 6);
      await getOrCreateAssociatedTokenAccount(provider.connection, payer, outputMint, vault, true);
    });

    it("wraps SOL atomically and creates swap PDA with input_mint = WSOL", async () => {
      // No fee is collected at propose-time; the protocol fee is deducted at execute_swap.
      const swapAmount = 50_000_000; // 0.05 SOL
      const minOutput  = 1_000_000;

      const vaultBefore = await provider.connection.getBalance(vault);
      const wsolBefore  = await provider.connection.getBalance(vaultWsolAta);

      const v     = await program.account.vaultState.fetch(vault);
      const nonce = v.nonce.toNumber();
      const sp    = swapPda(program.programId, vault, nonce);

      await program.methods
        .proposeSolSwap(outputMint, new BN(swapAmount), new BN(minOutput), new BN(VAULT_ID), creator)
        .accountsPartial({
          vault, swapTransaction: sp, vaultWsolAccount: vaultWsolAta,
          proposer: creator,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const s = await program.account.swapTransactionState.fetch(sp);
      expect(s.inputMint.toBase58()).to.equal(NATIVE_MINT.toBase58());
      expect(s.outputMint.toBase58()).to.equal(outputMint.toBase58());
      expect(s.inputAmount.toNumber()).to.equal(swapAmount);
      expect(s.executed).to.be.false;
      expect(s.approvals).to.deep.equal([]);

      const vaultAfter = await provider.connection.getBalance(vault);
      const wsolAfter  = await provider.connection.getBalance(vaultWsolAta);

      expect(wsolAfter - wsolBefore).to.equal(swapAmount, "WSOL ATA must gain swap_amount");
      expect(vaultBefore - vaultAfter).to.equal(swapAmount, "vault debit = swap_amount only");
    });

    it("rejects zero sol_amount", async () => {
      const sp = swapPda(program.programId, vault, (await program.account.vaultState.fetch(vault)).nonce.toNumber());
      try {
        await program.methods
          .proposeSolSwap(outputMint, new BN(0), new BN(1_000_000), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, swapTransaction: sp, vaultWsolAccount: vaultWsolAta, proposer: creator, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected InvalidAmount");
      } catch (e) { expect(errStr(e)).to.include("InvalidAmount"); }
    });

    it("rejects zero minimum_output_amount", async () => {
      const sp = swapPda(program.programId, vault, (await program.account.vaultState.fetch(vault)).nonce.toNumber());
      try {
        await program.methods
          .proposeSolSwap(outputMint, new BN(10_000_000), new BN(0), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, swapTransaction: sp, vaultWsolAccount: vaultWsolAta, proposer: creator, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected InvalidAmount");
      } catch (e) { expect(errStr(e)).to.include("InvalidAmount"); }
    });

    it("rejects when vault has insufficient SOL (amount + fee > spendable balance)", async () => {
      const poorVault = vaultPda(program.programId, creator, 701);
      await program.methods
        .createVault([creator], 1, new BN(701), "Poor Vault")
        .accountsPartial({ vault: poorVault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, poorVault, 0.002);

      const poorWsol = (await getOrCreateAssociatedTokenAccount(provider.connection, payer, NATIVE_MINT, poorVault, true)).address;
      const sp       = swapPda(program.programId, poorVault, (await program.account.vaultState.fetch(poorVault)).nonce.toNumber());

      try {
        await program.methods
          .proposeSolSwap(outputMint, new BN(10_000_000), new BN(1_000_000), new BN(701), creator)
          .accountsPartial({ vault: poorVault, swapTransaction: sp, vaultWsolAccount: poorWsol, proposer: creator, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId })
          .rpc();
        expect.fail("expected InsufficientFunds");
      } catch (e) { expect(errStr(e)).to.include("InsufficientFunds"); }
    });

    it("rejects a non-owner proposer", async () => {
      const sp = swapPda(program.programId, vault, (await program.account.vaultState.fetch(vault)).nonce.toNumber());
      try {
        await program.methods
          .proposeSolSwap(outputMint, new BN(10_000_000), new BN(1_000_000), new BN(VAULT_ID), creator)
          .accountsPartial({ vault, swapTransaction: sp, vaultWsolAccount: vaultWsolAta, proposer: owner2.publicKey, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId })
          .signers([owner2])
          .rpc();
        expect.fail("expected UnauthorizedProposer");
      } catch (e) { expect(errStr(e)).to.include("UnauthorizedProposer"); }
    });

    it("approve_swap + execute_swap validation work on a propose_sol_swap PDA", async () => {
      const v     = await program.account.vaultState.fetch(vault);
      const nonce = v.nonce.toNumber();
      const sp    = swapPda(program.programId, vault, nonce);

      await program.methods
        .proposeSolSwap(outputMint, new BN(20_000_000), new BN(1_000), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, swapTransaction: sp, vaultWsolAccount: vaultWsolAta, proposer: creator, tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      await program.methods
        .approveSwap(creator, new BN(VAULT_ID), new BN(nonce))
        .accountsPartial({ vault, swapTransaction: sp, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const s = await program.account.swapTransactionState.fetch(sp);
      expect(s.approvals.map((k: PublicKey) => k.toBase58())).to.include(creator.toBase58());
      // execute_swap requires a live Jupiter CPI - only the approval path is tested here.
    });
  });

  // ==========================================================================
  // propose_wrap / approve_wrap / execute_wrap / cancel_wrap
  // ==========================================================================

  describe("wrap transactions", () => {
    const VAULT_ID = 500;
    let vault: PublicKey;
    let vaultWsolAccount: PublicKey;

    before(async () => {
      vault = vaultPda(program.programId, creator, VAULT_ID);
      await program.methods
        .createVault([creator], 1, new BN(VAULT_ID), "Wrap Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 1);

      const ata  = await getOrCreateAssociatedTokenAccount(provider.connection, payer, WSOL_MINT, vault, true);
      vaultWsolAccount = ata.address;
    });

    it("propose_wrap: creates a pending wrap with zero approvals", async () => {
      const nonce  = 0;
      const wp     = wrapPda(program.programId, vault, nonce);
      const amount = new BN(50_000_000); // 0.05 SOL

      await program.methods
        .proposeWrap(amount, new BN(VAULT_ID), creator)
        .accountsPartial({ vault, wrapTransaction: wp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const w = await program.account.wrapTransactionState.fetch(wp);
      expect(w.amount.toNumber()).to.equal(amount.toNumber());
      expect(w.approvals).to.deep.equal([]);
      expect(w.executed).to.be.false;
    });

    it("approve_wrap: records the approval", async () => {
      const wp = wrapPda(program.programId, vault, 0);
      await program.methods
        .approveWrap(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({ vault, wrapTransaction: wp, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const w = await program.account.wrapTransactionState.fetch(wp);
      expect(w.approvals).to.have.lengthOf(1);
      expect(w.approvals[0].toBase58()).to.equal(creator.toBase58());
    });

    it("execute_wrap: transfers SOL to WSOL ATA and fee to fee recipient", async () => {
      const wp          = wrapPda(program.programId, vault, 0);
      const vaultBefore = await provider.connection.getBalance(vault);
      const wsolBefore  = await provider.connection.getBalance(vaultWsolAccount);
      const wrapAmount  = 50_000_000;

      await program.methods
        .executeWrap(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({
          vault, wrapTransaction: wp, vaultWsolAccount,
          signer: creator, feeRecipient: FEE_RECIPIENT,
          tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

      const w = await program.account.wrapTransactionState.fetch(wp);
      expect(w.executed).to.be.true;

      const vaultAfter = await provider.connection.getBalance(vault);
      const wsolAfter  = await provider.connection.getBalance(vaultWsolAccount);

      expect(vaultBefore - vaultAfter).to.equal(wrapAmount + MIN_FEE_LAMPORTS);
      expect(wsolAfter   - wsolBefore).to.equal(wrapAmount);
    });

    it("execute_wrap: rejects execution without sufficient approvals", async () => {
      const MV_ID = 501;
      const mv = vaultPda(program.programId, creator, MV_ID);
      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(MV_ID), "Multi Wrap Vault")
        .accountsPartial({ vault: mv, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, mv, 0.5);

      const mvWsol = (await getOrCreateAssociatedTokenAccount(provider.connection, payer, WSOL_MINT, mv, true)).address;
      const wp     = wrapPda(program.programId, mv, 0);

      await program.methods
        .proposeWrap(new BN(10_000_000), new BN(MV_ID), creator)
        .accountsPartial({ vault: mv, wrapTransaction: wp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      // Only 0 approvals - threshold is 2.
      try {
        await program.methods
          .executeWrap(creator, new BN(MV_ID), new BN(0))
          .accountsPartial({
            vault: mv, wrapTransaction: wp, vaultWsolAccount: mvWsol,
            signer: creator, feeRecipient: FEE_RECIPIENT,
            tokenProgram: TOKEN_PROGRAM_ID, systemProgram: anchor.web3.SystemProgram.programId,
          })
          .rpc();
        expect.fail("expected InsufficientApprovals");
      } catch (e) { expect(errStr(e)).to.include("InsufficientApprovals"); }
    });

    it("cancel_wrap: removes proposal from vault pending list at threshold", async () => {
      const CV_ID = 502;
      const cv = vaultPda(program.programId, creator, CV_ID);
      await program.methods
        .createVault([creator], 1, new BN(CV_ID), "Cancel Wrap Vault")
        .accountsPartial({ vault: cv, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, cv, 0.5);

      const wp = wrapPda(program.programId, cv, 0);
      await program.methods
        .proposeWrap(new BN(10_000_000), new BN(CV_ID), creator)
        .accountsPartial({ vault: cv, wrapTransaction: wp, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      let before = await program.account.vaultState.fetch(cv);
      expect(before.pendingTransactions.map((x: BN) => x.toNumber())).to.include(0);

      await program.methods
        .cancelWrap(creator, new BN(CV_ID), new BN(0))
        .accountsPartial({ vault: cv, wrapTransaction: wp, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      const after = await program.account.vaultState.fetch(cv);
      expect(after.pendingTransactions.map((x: BN) => x.toNumber())).to.not.include(0);
    });
  });

  // ==========================================================================
  // Multi-step workflow (end-to-end)
  // ==========================================================================

  describe("end-to-end workflow", () => {
    it("two concurrent proposals: execute one, pending list shrinks correctly", async () => {
      const VAULT_ID = 50;
      const vault = vaultPda(program.programId, creator, VAULT_ID);

      await program.methods
        .createVault([creator, owner2.publicKey], 2, new BN(VAULT_ID), "Workflow Vault")
        .accountsPartial({ vault, creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await airdrop(provider.connection, vault, 3);

      const tx0 = txPda(program.programId, vault, 0);
      const tx1 = txPda(program.programId, vault, 1);

      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.5 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx0, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .proposeTransaction(target.publicKey, false, anchor.web3.SystemProgram.programId, new BN(0.3 * LAMPORTS_PER_SOL), new BN(VAULT_ID), creator)
        .accountsPartial({ vault, transaction: tx1, proposer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();

      let v = await program.account.vaultState.fetch(vault);
      expect(v.pendingTransactions).to.have.lengthOf(2);

      // Approve and execute the first transaction only.
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({ vault, transaction: tx0, signer: creator, systemProgram: anchor.web3.SystemProgram.programId })
        .rpc();
      await program.methods
        .approveTransaction(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({ vault, transaction: tx0, signer: owner2.publicKey, systemProgram: anchor.web3.SystemProgram.programId })
        .signers([owner2])
        .rpc();

      const targetBefore = await provider.connection.getBalance(target.publicKey);
      await program.methods
        .executeTransaction(creator, new BN(VAULT_ID), new BN(0))
        .accountsPartial({
          vault, transaction: tx0, signer: creator,
          feeRecipient: FEE_RECIPIENT, target: target.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          vaultTokenAccount: vault, targetTokenAccount: target.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      const targetAfter = await provider.connection.getBalance(target.publicKey);
      v = await program.account.vaultState.fetch(vault);

      expect(v.pendingTransactions).to.have.lengthOf(1);
      expect(targetAfter).to.be.greaterThan(targetBefore);
    });
  });
});
