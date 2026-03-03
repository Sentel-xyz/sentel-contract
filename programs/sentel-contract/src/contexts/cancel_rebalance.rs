use crate::state::{BalancedVaultState, RebalanceProposalState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(vault_id: u64, proposal_nonce: u64)]
pub struct CancelRebalance<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    // No `close` - account stays alive while cancel votes accumulate
    #[account(
        mut,
        seeds = [
            b"rebalance_proposal",
            creator.key().as_ref(),
            &vault_id.to_le_bytes(),
            &proposal_nonce.to_le_bytes()
        ],
        bump
    )]
    pub rebalance_proposal: Account<'info, RebalanceProposalState>,

    pub creator: SystemAccount<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}
