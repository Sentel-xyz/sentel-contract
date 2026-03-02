use crate::{contexts::ExecuteWrap, errors::CustomError};
use anchor_lang::prelude::*;

pub fn execute_wrap(
    ctx: Context<ExecuteWrap>,
    _creator: Pubkey,
    _vault_id: u64,
    _wrap_nonce: u64,
) -> Result<()> {
    // Read all needed values before taking mutable borrows
    let signer_key = ctx.accounts.signer.key();
    let threshold = ctx.accounts.vault.threshold as usize;
    let owners = ctx.accounts.vault.owners.clone();
    let already_executed = ctx.accounts.wrap_transaction.executed;
    let expires_at = ctx.accounts.wrap_transaction.expires_at;
    let num_approvals = ctx.accounts.wrap_transaction.approvals.len();
    let amount = ctx.accounts.wrap_transaction.amount;
    let wrap_id = ctx.accounts.wrap_transaction.id;

    require!(
        owners.contains(&signer_key),
        CustomError::UnauthorizedProposer
    );

    require!(!already_executed, CustomError::TransactionAlreadyExecuted);

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp <= expires_at,
        CustomError::TransactionExpired
    );

    require!(
        num_approvals >= threshold,
        CustomError::InsufficientApprovals
    );

    let fee_amount = crate::MIN_FEE_LAMPORTS;
    let total_deduction = amount
        .checked_add(fee_amount)
        .ok_or(CustomError::InsufficientFunds)?;

    let vault_balance = ctx.accounts.vault.to_account_info().lamports();
    require!(
        vault_balance >= total_deduction,
        CustomError::InsufficientFunds
    );

    // Transfer lamports: vault -> WSOL ATA (SOL wrapping) + vault -> fee recipient
    // This is the core "wrap" operation: moving SOL into the WSOL token account's lamports.
    // The token account's `amount` field will be stale until sync_native is called separately,
    // but the lamports correctly reflect the wrapped balance.
    {
        let vault_info = ctx.accounts.vault.to_account_info();
        let wsol_info = ctx.accounts.vault_wsol_account.to_account_info();
        let fee_info = ctx.accounts.fee_recipient.to_account_info();

        // Use checked arithmetic for all lamport mutations to prevent underflow.
        let vault_lamports = vault_info.lamports();
        **vault_info.try_borrow_mut_lamports()? = vault_lamports
            .checked_sub(total_deduction)
            .ok_or(CustomError::InsufficientFunds)?;

        let wsol_lamports = wsol_info.lamports();
        **wsol_info.try_borrow_mut_lamports()? = wsol_lamports
            .checked_add(amount)
            .ok_or(CustomError::InsufficientFunds)?;

        let fee_lamports = fee_info.lamports();
        **fee_info.try_borrow_mut_lamports()? = fee_lamports
            .checked_add(fee_amount)
            .ok_or(CustomError::InsufficientFunds)?;
    }

    // Mark executed and remove from pending
    ctx.accounts.wrap_transaction.executed = true;
    ctx.accounts
        .vault
        .pending_transactions
        .retain(|&id| id != wrap_id);

    Ok(())
}
