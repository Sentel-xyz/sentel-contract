use crate::contexts::OpenBalancedVault;
use crate::errors::CustomError;
use crate::state::TokenAllocation;
use anchor_lang::prelude::*;

pub fn open_balanced_vault(
    ctx: Context<OpenBalancedVault>,
    vault_id: u64,
    owners: Vec<Pubkey>,
    threshold: u8,
    allocations: Vec<TokenAllocation>,
    name: String,
) -> Result<()> {
    let balanced_vault = &mut ctx.accounts.balanced_vault;

    require!(!owners.is_empty(), CustomError::EmptyOwners);
    require!(owners.len() <= 10, CustomError::TooManyOwners);
    require!(
        threshold > 0 && threshold <= owners.len() as u8,
        CustomError::InvalidThreshold
    );
    require!(name.len() <= 20, CustomError::NameTooLong);
    require!(!name.trim().is_empty(), CustomError::EmptyName);
    require!(allocations.len() <= 10, CustomError::TooManyAllocations);
    require!(!allocations.is_empty(), CustomError::InvalidAllocationTotal);

    let mut unique_owners = std::collections::HashSet::new();
    for owner in &owners {
        require!(unique_owners.insert(*owner), CustomError::DuplicateOwner);
    }

    let mut total_percentage: u32 = 0;
    let mut unique_mints = std::collections::HashSet::new();

    for allocation in &allocations {
        require!(
            allocation.percentage > 0 && allocation.percentage <= 10000,
            CustomError::InvalidPercentage
        );
        require!(
            unique_mints.insert(allocation.mint),
            CustomError::DuplicateMint
        );
        total_percentage += allocation.percentage as u32;
    }

    require!(
        total_percentage == 10000,
        CustomError::InvalidAllocationTotal
    );

    balanced_vault.creator = ctx.accounts.creator.key();
    balanced_vault.vault_id = vault_id;
    balanced_vault.owners = owners;
    balanced_vault.threshold = threshold;
    balanced_vault.allocations = allocations.clone();
    balanced_vault.is_active = true;
    balanced_vault.created_at = Clock::get()?.unix_timestamp;
    balanced_vault.name = name;
    balanced_vault.bump = ctx.bumps.balanced_vault;
    balanced_vault.nonce = 0;
    balanced_vault.pending_transactions = Vec::new();

    Ok(())
}
