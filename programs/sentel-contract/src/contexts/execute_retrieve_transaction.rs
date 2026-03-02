use crate::state::{BalancedVaultState, RetrieveTransactionState};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(vault_id: u64, transaction_nonce: u64)]
pub struct ExecuteRetrieveTransaction<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    #[account(
        mut,
        seeds = [
            b"retrieve_transaction",
            creator.key().as_ref(),
            &vault_id.to_le_bytes(),
            &transaction_nonce.to_le_bytes()
        ],
        bump,
        close = executor
    )]
    pub retrieve_transaction: Account<'info, RetrieveTransactionState>,

    // WSOL token account must be owned by the balanced vault PDA and use the canonical WSOL mint.
    #[account(
        mut,
        constraint = vault_wsol_account.owner == balanced_vault.key() @ crate::errors::CustomError::InvalidTokenAccount,
        constraint = vault_wsol_account.mint == Pubkey::from_str(crate::WSOL_MINT).unwrap() @ crate::errors::CustomError::InvalidMint,
    )]
    pub vault_wsol_account: Account<'info, TokenAccount>,

    /// CHECK: Native SOL recipient  receives unwrapped SOL after WSOL close
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,

    /// CHECK: Validated against hardcoded PROTOCOL_FEE_RECIPIENT address
    #[account(
        mut,
        constraint = fee_recipient.key() == Pubkey::from_str(crate::PROTOCOL_FEE_RECIPIENT).unwrap() @ crate::errors::CustomError::InvalidFeeRecipient
    )]
    pub fee_recipient: UncheckedAccount<'info>,

    pub creator: SystemAccount<'info>,

    #[account(mut)]
    pub executor: Signer<'info>,

    /// CHECK: Jupiter V6 program
    pub jupiter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}
