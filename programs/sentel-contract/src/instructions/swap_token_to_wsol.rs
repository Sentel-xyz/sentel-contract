use crate::contexts::SwapTokenToWsol;
use crate::errors::CustomError;
use crate::instructions::jupiter_account_meta;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use std::str::FromStr;

/// Executes a single Jupiter swap on behalf of the vault PDA (token -> WSOL).
/// Unlike rebalance_vault, this does NOT call sync_native  it is designed for
/// the retrieve flow where we sell non-WSOL tokens back to WSOL before the final
/// executeRetrieveTransaction call (which only needs to unwrap + distribute).
///
/// Any vault owner may call this. The vault PDA signs the Jupiter CPI.
pub fn swap_token_to_wsol<'info>(
    ctx: Context<'_, '_, '_, 'info, SwapTokenToWsol<'info>>,
    vault_id: u64,
    jupiter_swap_data: Vec<u8>,
    swap_account_count: u32,
) -> Result<()> {
    let balanced_vault = &ctx.accounts.balanced_vault;

    require!(
        balanced_vault.is_active,
        CustomError::BalancedVaultNotActive
    );

    // Must be called by a vault owner
    let executor_key = ctx.accounts.executor.key();
    require!(
        balanced_vault.owners.contains(&executor_key),
        CustomError::Unauthorized
    );

    let jupiter_program_id = Pubkey::from_str(crate::JUPITER_V6_PROGRAM_ID)
        .map_err(|_| CustomError::InvalidFeeRecipient)?;

    require!(
        ctx.accounts.jupiter_program.key() == jupiter_program_id,
        CustomError::InvalidFeeRecipient
    );

    let vault_id_bytes = vault_id.to_le_bytes();
    let creator_key = balanced_vault.creator;
    let vault_bump = balanced_vault.bump;
    let seeds = &[
        b"balanced_vault".as_ref(),
        creator_key.as_ref(),
        vault_id_bytes.as_ref(),
        &[vault_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let remaining_accounts = ctx.remaining_accounts;
    let count = swap_account_count as usize;

    require!(
        remaining_accounts.len() >= count,
        CustomError::MissingTokenAccounts
    );

    let swap_account_infos = &remaining_accounts[..count];
    let balanced_vault_key = ctx.accounts.balanced_vault.key();

    let swap_accounts: Vec<_> = swap_account_infos
        .iter()
        .map(|acc| jupiter_account_meta(acc, &balanced_vault_key))
        .collect();

    let jupiter_ix = Instruction {
        program_id: jupiter_program_id,
        accounts: swap_accounts,
        data: jupiter_swap_data,
    };

    invoke_signed(&jupiter_ix, swap_account_infos, signer_seeds)?;

    Ok(())
}
