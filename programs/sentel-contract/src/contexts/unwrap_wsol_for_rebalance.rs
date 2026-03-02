use crate::state::BalancedVaultState;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct UnwrapWsolForRebalance<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    /// WSOL ATA owned by the vault PDA.
    #[account(
        mut,
        constraint = vault_wsol_account.owner == balanced_vault.key() @ crate::errors::CustomError::InvalidTokenAccount,
        constraint = vault_wsol_account.mint == Pubkey::from_str(crate::WSOL_MINT).unwrap() @ crate::errors::CustomError::InvalidMint,
    )]
    pub vault_wsol_account: Account<'info, TokenAccount>,

    /// Creator is only used for PDA derivation.
    pub creator: SystemAccount<'info>,

    /// Any vault owner can trigger this.
    #[account(mut)]
    pub rebalancer: Signer<'info>,

    pub token_program: Program<'info, Token>,
}
