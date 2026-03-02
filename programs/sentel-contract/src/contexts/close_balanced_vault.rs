use crate::state::BalancedVaultState;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(vault_id: u64)]
pub struct CloseBalancedVault<'info> {
    #[account(
        mut,
        close = creator,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}
