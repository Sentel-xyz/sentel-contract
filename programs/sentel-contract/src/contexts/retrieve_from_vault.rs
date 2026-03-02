use crate::errors::CustomError;
use crate::state::BalancedVaultState;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct RetrieveFromVault<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    #[account(
        mut,
        constraint = vault_wsol_account.owner == balanced_vault.key() @ CustomError::InvalidTokenAccount,
        constraint = vault_wsol_account.mint == Pubkey::from_str(crate::WSOL_MINT).unwrap() @ CustomError::InvalidMint,
    )]
    pub vault_wsol_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: Jupiter V6 program
    pub jupiter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}
