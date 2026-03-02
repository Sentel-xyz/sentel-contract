use crate::contexts::UnwrapWsolForRebalance;
use crate::errors::CustomError;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount};

/// Closes the vault's WSOL ATA, returning all lamports to the vault PDA as native SOL.
/// Call this after rebalance swaps are complete to convert any remaining WSOL back to
/// native SOL (e.g. for a SOL allocation in the portfolio).
pub fn unwrap_wsol_for_rebalance(
    ctx: Context<UnwrapWsolForRebalance>,
    vault_id: u64,
) -> Result<()> {
    let balanced_vault = &ctx.accounts.balanced_vault;

    require!(
        balanced_vault.is_active,
        CustomError::BalancedVaultNotActive
    );

    let rebalancer_key = ctx.accounts.rebalancer.key();
    require!(
        balanced_vault.owners.contains(&rebalancer_key),
        CustomError::Unauthorized
    );

    let vault_id_bytes = vault_id.to_le_bytes();
    let creator_key = balanced_vault.creator;
    let seeds = &[
        b"balanced_vault".as_ref(),
        creator_key.as_ref(),
        vault_id_bytes.as_ref(),
        &[balanced_vault.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Close the WSOL ATA: all lamports (WSOL + rent) return to the vault PDA as native SOL.
    let close_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        CloseAccount {
            account: ctx.accounts.vault_wsol_account.to_account_info(),
            destination: ctx.accounts.balanced_vault.to_account_info(),
            authority: ctx.accounts.balanced_vault.to_account_info(),
        },
        signer_seeds,
    );
    token::close_account(close_ctx)?;

    Ok(())
}
