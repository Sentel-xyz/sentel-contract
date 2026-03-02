use crate::state::VaultState;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

#[derive(Accounts)]
#[instruction(creator_key: Pubkey, vault_id: u64)]
pub struct CloseVault<'info> {
    #[account(
        mut,
        seeds = [b"vault", creator_key.as_ref(), &vault_id.to_le_bytes()],
        bump,
        close = creator_signer
    )]
    pub vault: Account<'info, VaultState>,

    #[account(
        mut,
        constraint = creator_signer.key() == creator_key @ crate::errors::CustomError::UnauthorizedCreator
    )]
    pub creator_signer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}
