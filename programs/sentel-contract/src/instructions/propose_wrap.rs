use crate::{contexts::ProposeWrap, errors::CustomError};
use anchor_lang::prelude::*;

pub fn propose_wrap(
    ctx: Context<ProposeWrap>,
    amount: u64,
    _vault_id: u64,
    _creator: Pubkey,
) -> Result<()> {
    require!(amount > 0, CustomError::InvalidAmount);

    let vault = &mut ctx.accounts.vault;
    let wrap_transaction = &mut ctx.accounts.wrap_transaction;

    require!(
        vault.owners.contains(&ctx.accounts.proposer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(
        vault.pending_transactions.len() < 50,
        CustomError::TooManyPendingTransactions
    );

    let fee_amount = crate::MIN_FEE_LAMPORTS;
    let total_needed = amount
        .checked_add(fee_amount)
        .ok_or(CustomError::InsufficientFunds)?;

    let vault_info = vault.to_account_info();
    let vault_balance = vault_info.lamports();
    let rent_exempt = Rent::get()?.minimum_balance(vault_info.data_len());
    let available_balance = vault_balance.saturating_sub(rent_exempt);

    require!(
        available_balance >= total_needed,
        CustomError::InsufficientFunds
    );

    wrap_transaction.id = vault.nonce;
    wrap_transaction.amount = amount;
    wrap_transaction.proposer = ctx.accounts.proposer.key();
    wrap_transaction.approvals = vec![];
    wrap_transaction.cancellations = vec![];
    wrap_transaction.executed = false;

    let clock = Clock::get()?;
    wrap_transaction.created_at = clock.unix_timestamp;
    wrap_transaction.expires_at = clock
        .unix_timestamp
        .checked_add(7 * 24 * 60 * 60)
        .ok_or(CustomError::InvalidAmount)?;

    let current_nonce = vault.nonce;
    vault.pending_transactions.push(current_nonce);
    vault.nonce = vault
        .nonce
        .checked_add(1)
        .ok_or(CustomError::InvalidAmount)?;

    Ok(())
}
