use crate::state::{VaultState, WrapTransactionState};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, wrap_nonce: u64)]
pub struct ExecuteWrap<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"wrap", vault.key().as_ref(), &wrap_nonce.to_le_bytes()],
        bump
    )]
    pub wrap_transaction: Account<'info, WrapTransactionState>,

    // The vault's WSOL token account  must be owned by the vault PDA and use the canonical WSOL mint.
    // This prevents an attacker from substituting an arbitrary account to redirect lamports.
    #[account(
        mut,
        constraint = vault_wsol_account.owner == vault.key() @ crate::errors::CustomError::InvalidTokenAccount,
        constraint = vault_wsol_account.mint == Pubkey::from_str(crate::WSOL_MINT).unwrap() @ crate::errors::CustomError::InvalidMint,
    )]
    pub vault_wsol_account: Account<'info, TokenAccount>,

    #[account(mut, signer)]
    pub signer: Signer<'info>,

    /// CHECK: Validated against hardcoded PROTOCOL_FEE_RECIPIENT address
    #[account(
        mut,
        constraint = fee_recipient.key() == Pubkey::from_str(crate::PROTOCOL_FEE_RECIPIENT).unwrap() @ crate::errors::CustomError::InvalidFeeRecipient
    )]
    pub fee_recipient: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
