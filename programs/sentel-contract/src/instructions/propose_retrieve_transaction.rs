use crate::contexts::ProposeRetrieveTransaction;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

pub fn propose_retrieve_transaction(
    ctx: Context<ProposeRetrieveTransaction>,
    _vault_id: u64,
    transaction_nonce: u64,
    recipient: Pubkey,
) -> Result<()> {
    let balanced_vault = &mut ctx.accounts.balanced_vault;
    let retrieve_transaction = &mut ctx.accounts.retrieve_transaction;
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

    // Verify the nonce matches
    require!(
        transaction_nonce == balanced_vault.nonce,
        CustomError::InvalidNonce
    );

    // Block duplicate proposals  only one pending retrieval allowed at a time
    require!(
        balanced_vault.pending_transactions.is_empty(),
        CustomError::RetrievalAlreadyPending
    );

    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    let expiration_time = current_time + crate::TRANSACTION_EXPIRY_SECONDS; // 7 days

    // Initialize the retrieve transaction
    retrieve_transaction.id = transaction_nonce;
    retrieve_transaction.proposer = proposer.key();
    retrieve_transaction.recipient = recipient;
    retrieve_transaction.approvals = vec![];
    retrieve_transaction.cancellations = vec![];
    retrieve_transaction.executed = false;
    retrieve_transaction.created_at = current_time;
    retrieve_transaction.expires_at = expiration_time;

    // Add to pending transactions and increment nonce atomically.
    balanced_vault.pending_transactions.push(transaction_nonce);
    balanced_vault.nonce = balanced_vault
        .nonce
        .checked_add(1)
        .ok_or(CustomError::InvalidAmount)?;

    Ok(())
}
