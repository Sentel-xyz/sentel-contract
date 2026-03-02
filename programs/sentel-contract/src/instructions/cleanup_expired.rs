use crate::{contexts::CleanupExpired, errors::CustomError};
use anchor_lang::prelude::*;

pub fn cleanup_expired(
    ctx: Context<CleanupExpired>,
    _creator: Pubkey,
    _vault_id: u64,
    transaction_nonce: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let transaction = &ctx.accounts.transaction;
    let rent_receiver = &ctx.accounts.rent_receiver;

    require!(
        vault.owners.contains(&rent_receiver.key()),
        CustomError::UnauthorizedProposer
    );

    // Only unexecuted transactions can be cleaned up this way.
    require!(
        !transaction.executed,
        CustomError::TransactionAlreadyExecuted
    );

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp > transaction.expires_at,
        CustomError::TransactionNotExpired
    );

    vault
        .pending_transactions
        .retain(|&id| id != transaction_nonce);

    Ok(())
}
