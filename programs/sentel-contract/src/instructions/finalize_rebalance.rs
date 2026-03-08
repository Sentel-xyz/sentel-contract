use crate::contexts::FinalizeRebalance;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

/// Close the rebalance proposal PDA after all individual swaps have been executed
/// via `execute_rebalance_swap`. Rent is returned to the executor.
pub fn finalize_rebalance(
    ctx: Context<FinalizeRebalance>,
    _vault_id: u64,
    _proposal_nonce: u64,
) -> Result<()> {
    let rebalance_proposal = &ctx.accounts.rebalance_proposal;

    require!(
        rebalance_proposal.executed,
        CustomError::TransactionAlreadyExecuted // proposal not fully executed yet
    );

    require!(
        ctx.accounts
            .balanced_vault
            .owners
            .contains(&ctx.accounts.executor.key()),
        CustomError::Unauthorized
    );

    // Remove from pending_transactions  same cleanup as execute_rebalance does.
    let proposal_id = rebalance_proposal.id;
    ctx.accounts
        .balanced_vault
        .pending_transactions
        .retain(|&id| id != proposal_id);

    // PDA is closed by Anchor via `close = executor` on the account constraint.
    Ok(())
}
