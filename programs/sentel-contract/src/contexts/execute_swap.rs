use crate::state::{SwapTransactionState, VaultState};
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, swap_nonce: u64)]
pub struct ExecuteSwap<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"swap", vault.key().as_ref(), &swap_nonce.to_le_bytes()],
        bump,
        close = signer
    )]
    pub swap_transaction: Account<'info, SwapTransactionState>,

    #[account(
        mut,
        constraint = vault_input_token_account.owner == vault.key(),
        constraint = vault_input_token_account.mint == swap_transaction.input_mint
    )]
    pub vault_input_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = vault_output_token_account.owner == vault.key(),
        constraint = vault_output_token_account.mint == swap_transaction.output_mint
    )]
    pub vault_output_token_account: Account<'info, TokenAccount>,

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

    /// CHECK: Jupiter V6 program - JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4
    pub jupiter_program: UncheckedAccount<'info>,
}
