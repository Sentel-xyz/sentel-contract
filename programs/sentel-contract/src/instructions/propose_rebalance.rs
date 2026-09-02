use crate::contexts::ProposeRebalance;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

pub fn propose_rebalance(
    ctx: Context<ProposeRebalance>,
    _vault_id: u64,
    proposal_nonce: u64,
    payload_hashes: Vec<[u8; 32]>,
) -> Result<()> {
    let balanced_vault = &mut ctx.accounts.balanced_vault;
    let rebalance_proposal = &mut ctx.accounts.rebalance_proposal;
    let proposer = &ctx.accounts.proposer;

    // Verify proposer is an owner
    require!(
        balanced_vault.owners.contains(&proposer.key()),
        CustomError::Unauthorized
    );

    require!(
        balanced_vault.is_active,
        CustomError::BalancedVaultNotActive
    );

    // Verify the nonce matches the current vault nonce
    require!(
        proposal_nonce == balanced_vault.nonce,
        CustomError::InvalidNonce
    );

    // Only one pending transaction (retrieve or rebalance) allowed at a time
    require!(
        balanced_vault.pending_transactions.is_empty(),
        CustomError::RebalanceAlreadyPending
    );

    require!(!payload_hashes.is_empty(), CustomError::InvalidAmount);
    require!(payload_hashes.len() <= 10, CustomError::TooManySwaps);

    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    let expiration_time = current_time + crate::TRANSACTION_EXPIRY_SECONDS; // 7 days

    // Initialize the rebalance proposal
    rebalance_proposal.id = proposal_nonce;
    rebalance_proposal.proposer = proposer.key();
    rebalance_proposal.approvals = vec![];
    rebalance_proposal.cancellations = vec![];
    rebalance_proposal.executed = false;
    rebalance_proposal.created_at = current_time;
    rebalance_proposal.expires_at = expiration_time;
    rebalance_proposal.swaps_executed = 0;
    rebalance_proposal.total_swaps = payload_hashes.len() as u32;
    rebalance_proposal.payload_hashes = payload_hashes;

    // Track as pending and increment nonce atomically
    balanced_vault.pending_transactions.push(proposal_nonce);
    balanced_vault.nonce = balanced_vault
        .nonce
        .checked_add(1)
        .ok_or(CustomError::InvalidAmount)?;

    Ok(())
}
