use crate::contexts::UpdateAllocations;
use crate::errors::CustomError;
use crate::state::TokenAllocation;
use anchor_lang::prelude::*;

pub fn update_allocations(
    ctx: Context<UpdateAllocations>,
    _vault_id: u64,
    new_allocations: Vec<TokenAllocation>,
) -> Result<()> {
    let vault = &mut ctx.accounts.balanced_vault;

    require!(vault.is_active, CustomError::BalancedVaultNotActive);
    require!(
        vault.owners.contains(&ctx.accounts.updater.key()),
        CustomError::Unauthorized
    );
    // Block allocation changes while a retrieval is pending to prevent
    // inconsistency between stored allocations and in-flight proposals.
    require!(
        vault.pending_transactions.is_empty(),
        CustomError::VaultHasPendingTransactions
    );
    require!(
        !new_allocations.is_empty(),
        CustomError::InvalidAllocationTotal
    );
    require!(new_allocations.len() <= 10, CustomError::TooManyAllocations);

    let mut total: u32 = 0;
    let mut unique_mints = std::collections::HashSet::new();
    for alloc in &new_allocations {
        require!(
            alloc.percentage > 0 && alloc.percentage <= 10000,
            CustomError::InvalidPercentage
        );
        require!(unique_mints.insert(alloc.mint), CustomError::DuplicateMint);
        total += alloc.percentage as u32;
    }
    require!(total == 10000, CustomError::InvalidAllocationTotal);

    vault.allocations = new_allocations;
    Ok(())
}
