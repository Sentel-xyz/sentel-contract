use crate::state::BalancedVaultState;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct SwapTokenToWsol<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    pub creator: SystemAccount<'info>,

    #[account(mut)]
    pub executor: Signer<'info>,

    /// CHECK: Jupiter V6 program
    pub jupiter_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}
