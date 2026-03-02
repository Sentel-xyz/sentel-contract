use crate::state::BalancedVaultState;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct WrapSolForRebalance<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    // WSOL token account must be owned by the balanced vault PDA and use the canonical WSOL mint.
    #[account(
        mut,
        constraint = vault_wsol_account.owner == balanced_vault.key() @ crate::errors::CustomError::InvalidTokenAccount,
        constraint = vault_wsol_account.mint == Pubkey::from_str(crate::WSOL_MINT).unwrap() @ crate::errors::CustomError::InvalidMint,
    )]
    pub vault_wsol_account: Account<'info, TokenAccount>,

    /// Creator is only used for PDA derivation; any vault owner can trigger rebalance.
    pub creator: SystemAccount<'info>,

    /// The vault owner who is initiating the rebalance (must be an owner of balanced_vault).
    #[account(mut)]
    pub rebalancer: Signer<'info>,

    /// CHECK: Validated against hardcoded PROTOCOL_FEE_RECIPIENT address
    #[account(
        mut,
        constraint = fee_recipient.key() == Pubkey::from_str(crate::PROTOCOL_FEE_RECIPIENT).unwrap() @ crate::errors::CustomError::InvalidFeeRecipient
    )]
    pub fee_recipient: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}
