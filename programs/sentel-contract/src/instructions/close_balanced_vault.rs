use crate::contexts::CloseBalancedVault;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

const MAX_SOL_FOR_CLOSURE: u64 = 300_000_000; // 0.3 SOL in lamports

pub fn close_balanced_vault(ctx: Context<CloseBalancedVault>, _vault_id: u64) -> Result<()> {
    // Read lamports before taking the mutable borrow
    let vault_lamports = ctx.accounts.balanced_vault.to_account_info().lamports();

    let balanced_vault = &mut ctx.accounts.balanced_vault;

    require!(
        balanced_vault.is_active,
        CustomError::BalancedVaultNotActive
    );

    // Must have no pending transactions
    require!(
        balanced_vault.pending_transactions.is_empty(),
        CustomError::VaultHasPendingTransactions
    );

    // Vault SOL balance must be strictly below 0.3 SOL (excluding rent for this account itself,
    // which Anchor will reclaim when the account is closed via `close = creator`).
    require!(
        vault_lamports < MAX_SOL_FOR_CLOSURE,
        CustomError::VaultBalanceTooHighForClosure
    );

    balanced_vault.is_active = false;

    Ok(())
}
