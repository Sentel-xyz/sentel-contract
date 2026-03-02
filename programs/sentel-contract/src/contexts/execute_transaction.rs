use crate::state::{TransactionState, VaultState};
use anchor_lang::prelude::*;
use std::str::FromStr;

#[derive(Accounts)]
#[instruction(creator: Pubkey, vault_id: u64, transaction_nonce: u64)]
pub struct ExecuteTransaction<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator.as_ref(), &vault_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        mut,
        seeds = [b"transaction", vault.key().as_ref(), &transaction_nonce.to_le_bytes()],
        bump,
        close = signer
    )]
    pub transaction: Account<'info, TransactionState>,

    pub signer: Signer<'info>,

    /// CHECK: Target account is validated against transaction.target via constraint
    #[account(
        mut,
        constraint = target.key() == transaction.target @ crate::errors::CustomError::InvalidTarget
    )]
    pub target: UncheckedAccount<'info>,

    /// CHECK: Fee recipient is validated against the hardcoded PROTOCOL_FEE_RECIPIENT via constraint
    #[account(
        mut,
        constraint = fee_recipient.key() == Pubkey::from_str(crate::PROTOCOL_FEE_RECIPIENT).unwrap() @ crate::errors::CustomError::InvalidFeeRecipient
    )]
    pub fee_recipient: UncheckedAccount<'info>,

    // Optional token accounts for SPL token transfers
    /// CHECK: Token account validated in instruction when token_type is true
    #[account(mut)]
    pub vault_token_account: UncheckedAccount<'info>,

    /// CHECK: Token account validated in instruction when token_type is true
    #[account(mut)]
    pub target_token_account: UncheckedAccount<'info>,

    /// CHECK: Token program validated in instruction when token_type is true
    pub token_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}
