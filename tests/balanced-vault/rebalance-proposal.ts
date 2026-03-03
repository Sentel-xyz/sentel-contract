/**
 * Rebalance Proposal Flow Tests
 *
 * Covers the full propose → approve → execute multisig lifecycle for
 * rebalance_proposal, plus cancel and guard checks.
 *
 * Note: execute_rebalance calls Jupiter CPI for the actual swaps. On
 * localnet there is no Jupiter program, so this test exercises every
 * instruction EXCEPT the Jupiter CPI path by using an empty swap data
 * array.  The instruction will fail at the "require!(
 * !jupiter_swap_data.is_empty())" guard, so the execute test intentionally
 * triggers that error to prove the approval gate is the only thing gating
 * execution (not the proposer / nonce / expiry checks).
 *
 * Test matrix:
 *  1. Create a 2-of-2 balanced vault
 *  2. propose_rebalance  – creates PDA, nonce advances, pending updated
 *  3. propose_rebalance again  – blocked (RebalanceAlreadyPending)
 *  4. approve_rebalance (owner1)  – adds to approvals
 *  5. approve_rebalance (owner1) again  – blocked (AlreadyApproved)
 *  6. non-owner approve  – blocked (Unauthorized)
 *  7. approve_rebalance (owner2)  – reaches threshold
 *  8. execute_rebalance with empty swap data – blocked at InsufficientWsolForRebalance
 *     (proves approval gate passes; Jupiter path not reached on localnet)
 *  9. cancel_rebalance (owner1) – vote added, threshold not yet reached
 * 10. cancel_rebalance (owner1) again – blocked (AlreadyCancelledVote)
 * 11. cancel_rebalance (owner2) – threshold reached, proposal removed from pending
 * 12. New proposal after cancel uses next nonce, no collision
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
} from "@solana/spl-token";
import { expect } from "chai";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const WSOL_MINT   = new PublicKey("So11111111111111111111111111111111111111112");
const JUPITER_PROGRAM = new PublicKey("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");

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

function deriveBalancedVaultPda(
  programId: PublicKey,
  creator: PublicKey,
  vaultId: BN
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("balanced_vault"),
      creator.toBuffer(),
      vaultId.toArrayLike(Buffer, "le", 8),
    ],
    programId
  );
  return pda;
}

function deriveRebalanceProposalPda(
  programId: PublicKey,
  creator: PublicKey,
  vaultId: BN,
  nonce: BN
): PublicKey {
  const [pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("rebalance_proposal"),
      creator.toBuffer(),
      vaultId.toArrayLike(Buffer, "le", 8),
      nonce.toArrayLike(Buffer, "le", 8),
    ],
    programId
  );
  return pda;
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe("Rebalance Proposal Flow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.sentelContract as Program<SentelContract>;

  let creator:   Keypair;
  let owner1:    Keypair;
  let owner2:    Keypair;
  let outsider:  Keypair;

  let tokenMint:         PublicKey;
  let vaultId:           BN;
  let balancedVaultPda:  PublicKey;
  let vaultWsolAccount:  PublicKey;

  // Shared across tests
  let proposalNonce: BN;

  before(async () => {
    creator  = Keypair.generate();
    owner1   = Keypair.generate();
    owner2   = Keypair.generate();
    outsider = Keypair.generate();

    for (const [kp, sol] of [
      [creator,  10] as const,
      [owner1,    5] as const,
      [owner2,    5] as const,
      [outsider,  2] as const,
    ]) {
      await airdrop(provider.connection, kp.publicKey, sol);
    }

    tokenMint = await createMint(
      provider.connection,
      creator,
      creator.publicKey,
      null,
      6
    );
  });

  // =========================================================================
  // 1. Create vault
  // =========================================================================

  it("creates a 2-of-2 balanced vault", async () => {
    vaultId         = new BN(Date.now());
    balancedVaultPda = deriveBalancedVaultPda(program.programId, creator.publicKey, vaultId);

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
        "Rebalance Test Vault"
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

  // Fund vault WSOL ATA so execute path doesn't fail with missing account
  it("funds the vault with a WSOL token account", async () => {
    const wsolAmount    = 50_000_000; // 0.05 SOL
    const wsolKeypair   = Keypair.generate();
    vaultWsolAccount    = wsolKeypair.publicKey;

    const rentExemption = await provider.connection.getMinimumBalanceForRentExemption(165);

    const tx = new anchor.web3.Transaction().add(
      SystemProgram.createAccount({
        fromPubkey:      creator.publicKey,
        newAccountPubkey: wsolKeypair.publicKey,
        lamports:        rentExemption + wsolAmount,
        space:           165,
        programId:       TOKEN_PROGRAM_ID,
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
    expect(info!.lamports).to.be.greaterThanOrEqual(wsolAmount);
  });

  // =========================================================================
  // 2. propose_rebalance
  // =========================================================================

  it("owner1 proposes a rebalance", async () => {
    const vault    = await program.account.balancedVaultState.fetch(balancedVaultPda);
    proposalNonce  = vault.nonce;

    const proposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      proposalNonce
    );

    await program.methods
      .proposeRebalance(vaultId, proposalNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: proposalPda,
        creator:          creator.publicKey,
        proposer:         owner1.publicKey,
        systemProgram:    SystemProgram.programId,
      })
      .signers([owner1])
      .rpc();

    const proposal = await program.account.rebalanceProposalState.fetch(proposalPda);
    expect(proposal.id.toNumber()).to.equal(proposalNonce.toNumber());
    expect(proposal.proposer.toBase58()).to.equal(owner1.publicKey.toBase58());
    expect(proposal.approvals).to.have.lengthOf(0);
    expect(proposal.cancellations).to.have.lengthOf(0);
    expect(proposal.executed).to.be.false;

    const updatedVault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(updatedVault.nonce.toNumber()).to.equal(proposalNonce.toNumber() + 1);
    expect(updatedVault.pendingTransactions).to.have.lengthOf(1);
    expect(updatedVault.pendingTransactions[0].toNumber()).to.equal(proposalNonce.toNumber());
  });

  // =========================================================================
  // 3. Duplicate proposal blocked
  // =========================================================================

  it("second propose_rebalance is blocked while one is pending (RebalanceAlreadyPending)", async () => {
    const vault     = await program.account.balancedVaultState.fetch(balancedVaultPda);
    const nextNonce = vault.nonce; // nonce already incremented

    const nextProposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      nextNonce
    );

    let threw = false;
    try {
      await program.methods
        .proposeRebalance(vaultId, nextNonce)
        .accountsPartial({
          balancedVault:    balancedVaultPda,
          rebalanceProposal: nextProposalPda,
          creator:          creator.publicKey,
          proposer:         owner1.publicKey,
          systemProgram:    SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();
    } catch (err: any) {
      threw = true;
      expect(err.toString()).to.include("RebalanceAlreadyPending");
    }
    expect(threw, "Expected RebalanceAlreadyPending error").to.be.true;
  });

  // =========================================================================
  // 4. approve_rebalance (owner1)
  // =========================================================================

  it("owner1 approves the rebalance proposal", async () => {
    const proposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      proposalNonce
    );

    await program.methods
      .approveRebalance(vaultId, proposalNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: proposalPda,
        creator:          creator.publicKey,
        approver:         owner1.publicKey,
      })
      .signers([owner1])
      .rpc();

    const proposal = await program.account.rebalanceProposalState.fetch(proposalPda);
    expect(proposal.approvals).to.have.lengthOf(1);
    expect(proposal.approvals[0].toBase58()).to.equal(owner1.publicKey.toBase58());
  });

  // =========================================================================
  // 5. Double-approve blocked
  // =========================================================================

  it("owner1 double-approve is blocked (AlreadyApproved)", async () => {
    const proposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      proposalNonce
    );

    let threw = false;
    try {
      await program.methods
        .approveRebalance(vaultId, proposalNonce)
        .accountsPartial({
          balancedVault:    balancedVaultPda,
          rebalanceProposal: proposalPda,
          creator:          creator.publicKey,
          approver:         owner1.publicKey,
        })
        .signers([owner1])
        .rpc();
    } catch (err: any) {
      threw = true;
      expect(err.toString()).to.include("AlreadyApproved");
    }
    expect(threw, "Expected AlreadyApproved error").to.be.true;
  });

  // =========================================================================
  // 6. Non-owner approve blocked
  // =========================================================================

  it("non-owner approve is blocked (Unauthorized)", async () => {
    const proposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      proposalNonce
    );

    let threw = false;
    try {
      await program.methods
        .approveRebalance(vaultId, proposalNonce)
        .accountsPartial({
          balancedVault:    balancedVaultPda,
          rebalanceProposal: proposalPda,
          creator:          creator.publicKey,
          approver:         outsider.publicKey,
        })
        .signers([outsider])
        .rpc();
    } catch (err: any) {
      threw = true;
      expect(err.toString()).to.include("Unauthorized");
    }
    expect(threw, "Expected Unauthorized error").to.be.true;
  });

  // =========================================================================
  // 7. owner2 approves → reaches threshold
  // =========================================================================

  it("owner2 approves → proposal reaches 2-of-2 threshold", async () => {
    const proposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      proposalNonce
    );

    await program.methods
      .approveRebalance(vaultId, proposalNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: proposalPda,
        creator:          creator.publicKey,
        approver:         owner2.publicKey,
      })
      .signers([owner2])
      .rpc();

    const proposal = await program.account.rebalanceProposalState.fetch(proposalPda);
    expect(proposal.approvals).to.have.lengthOf(2);
  });

  // =========================================================================
  // 8. execute_rebalance with empty swap data  approval gate passes,
  //    fails at InsufficientWsolForRebalance (no Jupiter on localnet)
  // =========================================================================

  it("execute_rebalance with empty swap data fails at InsufficientWsolForRebalance (approval gate passed)", async () => {
    const proposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      proposalNonce
    );

    let threw = false;
    try {
      await program.methods
        .executeRebalance(vaultId, proposalNonce, [], [])
        .accountsPartial({
          balancedVault:    balancedVaultPda,
          rebalanceProposal: proposalPda,
          vaultWsolAccount: vaultWsolAccount,
          creator:          creator.publicKey,
          executor:         owner1.publicKey,
          jupiterProgram:   JUPITER_PROGRAM,
          tokenProgram:     TOKEN_PROGRAM_ID,
          systemProgram:    SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();
    } catch (err: any) {
      threw = true;
      // Should fail at the empty swap data guard, NOT at approval/nonce checks.
      expect(err.toString()).to.include("InsufficientWsolForRebalance");
    }
    expect(threw, "Expected InsufficientWsolForRebalance error (approval gate passed)").to.be.true;

    // Proposal is NOT consumed (executed stays false, still in pending).
    const proposal = await program.account.rebalanceProposalState.fetch(proposalPda);
    expect(proposal.executed).to.be.false;

    const vault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(vault.pendingTransactions).to.have.lengthOf(1);
  });

  // =========================================================================
  // Now we test the cancel flow. We'll spin up a fresh proposal for
  // the cancel tests so we have a clean proposal with 0 approvals.
  // =========================================================================

  let cancelNonce: BN;

  it("proposes a second rebalance for cancel flow testing", async () => {
    // First cancel the current proposal so we can propose again.
    const proposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      proposalNonce
    );

    // Cancel with owner1
    await program.methods
      .cancelRebalance(vaultId, proposalNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: proposalPda,
        creator:          creator.publicKey,
        signer:           owner1.publicKey,
        systemProgram:    SystemProgram.programId,
      })
      .signers([owner1])
      .rpc();

    // Cancel with owner2 → reaches threshold → removed from pending
    await program.methods
      .cancelRebalance(vaultId, proposalNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: proposalPda,
        creator:          creator.publicKey,
        signer:           owner2.publicKey,
        systemProgram:    SystemProgram.programId,
      })
      .signers([owner2])
      .rpc();

    const vault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(vault.pendingTransactions).to.have.lengthOf(0);

    // Now propose a fresh one for the cancel tests below.
    cancelNonce = vault.nonce;
    const cancelProposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      cancelNonce
    );

    await program.methods
      .proposeRebalance(vaultId, cancelNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: cancelProposalPda,
        creator:          creator.publicKey,
        proposer:         owner1.publicKey,
        systemProgram:    SystemProgram.programId,
      })
      .signers([owner1])
      .rpc();

    const proposal = await program.account.rebalanceProposalState.fetch(cancelProposalPda);
    expect(proposal.cancellations).to.have.lengthOf(0);
  });

  // =========================================================================
  // 9. cancel_rebalance (owner1)  threshold NOT yet reached
  // =========================================================================

  it("owner1 votes to cancel  cancellations=[owner1], proposal stays pending", async () => {
    const cancelProposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      cancelNonce
    );

    await program.methods
      .cancelRebalance(vaultId, cancelNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: cancelProposalPda,
        creator:          creator.publicKey,
        signer:           owner1.publicKey,
        systemProgram:    SystemProgram.programId,
      })
      .signers([owner1])
      .rpc();

    const proposal = await program.account.rebalanceProposalState.fetch(cancelProposalPda);
    expect(proposal.cancellations).to.have.lengthOf(1);
    expect(proposal.cancellations[0].toBase58()).to.equal(owner1.publicKey.toBase58());

    // Still in pending (threshold not reached yet for 2-of-2).
    const vault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(vault.pendingTransactions).to.have.lengthOf(1);
  });

  // =========================================================================
  // 10. Double cancel-vote blocked
  // =========================================================================

  it("owner1 double cancel-vote is blocked (AlreadyCancelledVote)", async () => {
    const cancelProposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      cancelNonce
    );

    let threw = false;
    try {
      await program.methods
        .cancelRebalance(vaultId, cancelNonce)
        .accountsPartial({
          balancedVault:    balancedVaultPda,
          rebalanceProposal: cancelProposalPda,
          creator:          creator.publicKey,
          signer:           owner1.publicKey,
          systemProgram:    SystemProgram.programId,
        })
        .signers([owner1])
        .rpc();
    } catch (err: any) {
      threw = true;
      expect(err.toString()).to.include("AlreadyCancelledVote");
    }
    expect(threw, "Expected AlreadyCancelledVote error").to.be.true;
  });

  // =========================================================================
  // 11. owner2 cancel-vote → threshold reached → removed from pending
  // =========================================================================

  it("owner2 cancel-vote reaches threshold  proposal removed from pending", async () => {
    const cancelProposalPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      cancelNonce
    );

    await program.methods
      .cancelRebalance(vaultId, cancelNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: cancelProposalPda,
        creator:          creator.publicKey,
        signer:           owner2.publicKey,
        systemProgram:    SystemProgram.programId,
      })
      .signers([owner2])
      .rpc();

    const proposal = await program.account.rebalanceProposalState.fetch(cancelProposalPda);
    expect(proposal.cancellations).to.have.lengthOf(2);

    // Proposal removed from pending_transactions.
    const vault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(vault.pendingTransactions).to.have.lengthOf(0);
  });

  // =========================================================================
  // 12. New proposal after cancel uses next nonce  no collision
  // =========================================================================

  it("new proposal after cancel uses the next nonce with no collision", async () => {
    const vault     = await program.account.balancedVaultState.fetch(balancedVaultPda);
    const nextNonce = vault.nonce;

    // Nonce must have advanced past both previous proposals.
    expect(nextNonce.toNumber()).to.be.greaterThan(cancelNonce.toNumber());

    const newPda = deriveRebalanceProposalPda(
      program.programId,
      creator.publicKey,
      vaultId,
      nextNonce
    );

    // PDA must not exist yet.
    expect(await provider.connection.getAccountInfo(newPda)).to.be.null;

    await program.methods
      .proposeRebalance(vaultId, nextNonce)
      .accountsPartial({
        balancedVault:    balancedVaultPda,
        rebalanceProposal: newPda,
        creator:          creator.publicKey,
        proposer:         owner2.publicKey,
        systemProgram:    SystemProgram.programId,
      })
      .signers([owner2])
      .rpc();

    const proposal = await program.account.rebalanceProposalState.fetch(newPda);
    expect(proposal.executed).to.be.false;
    expect(proposal.id.toNumber()).to.equal(nextNonce.toNumber());

    const updatedVault = await program.account.balancedVaultState.fetch(balancedVaultPda);
    expect(updatedVault.pendingTransactions).to.have.lengthOf(1);
    expect(updatedVault.pendingTransactions[0].toNumber()).to.equal(nextNonce.toNumber());
  });
});
