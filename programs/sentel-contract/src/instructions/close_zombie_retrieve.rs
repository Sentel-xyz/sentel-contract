use crate::contexts::CloseZombieRetrieve;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

pub fn close_zombie_retrieve(
    ctx: Context<CloseZombieRetrieve>,
    _vault_id: u64,
    _transaction_nonce: u64,
) -> Result<()> {
    let balanced_vault = &mut ctx.accounts.balanced_vault;
    let retrieve_transaction = &ctx.accounts.retrieve_transaction;
    let caller = &ctx.accounts.rent_receiver;

    // Only vault owners may call this
    require!(
        balanced_vault.owners.contains(&caller.key()),
        CustomError::Unauthorized
    );

    // Allow closing if:
    //   (a) already executed  -  rent reclaim after successful retrieve, OR
    //   (b) expired and not executed  -  unblocks vault from a permanently stale proposal
    let clock = Clock::get()?;
    let is_executed = retrieve_transaction.executed;
    let is_expired = clock.unix_timestamp > retrieve_transaction.expires_at;

    require!(
        is_executed || is_expired,
        CustomError::TransactionNotExpired
    );

    // Also clean up from pending_transactions in case it was left there somehow
    let nonce = retrieve_transaction.id;
    balanced_vault
        .pending_transactions
        .retain(|&id| id != nonce);

    Ok(())
}
