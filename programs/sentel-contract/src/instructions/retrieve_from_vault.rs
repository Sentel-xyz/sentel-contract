use crate::contexts::RetrieveFromVault;
use crate::errors::CustomError;
use crate::instructions::jupiter_account_meta;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use std::str::FromStr;

pub fn retrieve_from_vault<'info>(
    ctx: Context<'_, '_, '_, 'info, RetrieveFromVault<'info>>,
    vault_id: u64,
    jupiter_swap_data: Vec<Vec<u8>>,
    swap_account_counts: Vec<u32>,
) -> Result<()> {
    let balanced_vault = &ctx.accounts.balanced_vault;

    require!(
        balanced_vault.is_active,
        CustomError::BalancedVaultNotActive
    );

    // Only vault owners may trigger this.
    require!(
        balanced_vault.owners.contains(&ctx.accounts.creator.key()),
        CustomError::Unauthorized
    );

    let jupiter_program_id = Pubkey::from_str(crate::JUPITER_V6_PROGRAM_ID)
        .map_err(|_| CustomError::InvalidFeeRecipient)?;

    require!(
        ctx.accounts.jupiter_program.key() == jupiter_program_id,
        CustomError::InvalidFeeRecipient
    );

    require!(
        jupiter_swap_data.len() == swap_account_counts.len(),
        CustomError::MissingTokenAccounts
    );

    let vault_id_bytes = vault_id.to_le_bytes();
    let creator_key = balanced_vault.creator;
    let bump = balanced_vault.bump;
    let seeds = &[
        b"balanced_vault".as_ref(),
        creator_key.as_ref(),
        vault_id_bytes.as_ref(),
        &[bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let remaining_accounts = ctx.remaining_accounts;
    let balanced_vault_key = ctx.accounts.balanced_vault.key();
    let mut account_offset: usize = 0;

    for (i, swap_data) in jupiter_swap_data.iter().enumerate() {
        let count = swap_account_counts[i] as usize;

        require!(
            account_offset.saturating_add(count) <= remaining_accounts.len(),
            CustomError::MissingTokenAccounts
        );

        let swap_account_infos = &remaining_accounts[account_offset..account_offset + count];

        let swap_accounts: Vec<_> = swap_account_infos
            .iter()
            .map(|acc| jupiter_account_meta(acc, &balanced_vault_key))
            .collect();

        let jupiter_ix = Instruction {
            program_id: jupiter_program_id,
            accounts: swap_accounts,
            data: swap_data.clone(),
        };

        invoke_signed(&jupiter_ix, swap_account_infos, signer_seeds)?;
        account_offset += count;
    }

    Ok(())
}
