use crate::state::{BalancedVaultState, RebalanceProposalState};
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(vault_id: u64, proposal_nonce: u64)]
pub struct FinalizeRebalance<'info> {
    #[account(
        mut,
        seeds = [b"balanced_vault", creator.key().as_ref(), &vault_id.to_le_bytes()],
        bump = balanced_vault.bump,
        has_one = creator,
    )]
    pub balanced_vault: Account<'info, BalancedVaultState>,

    /// The rebalance proposal  must be fully executed (all swaps done). Closed here.
    #[account(
        mut,
        close = executor,
        seeds = [
            b"rebalance_proposal",
            creator.key().as_ref(),
            &vault_id.to_le_bytes(),
            &proposal_nonce.to_le_bytes()
        ],
        bump
    )]
    pub rebalance_proposal: Account<'info, RebalanceProposalState>,

    /// Creator is only used for PDA derivation.
    pub creator: SystemAccount<'info>,

    /// The vault owner finalizing the rebalance (must be an owner of balanced_vault).
    #[account(mut)]
    pub executor: Signer<'info>,

    pub system_program: Program<'info, System>,
}
