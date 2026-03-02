use crate::contexts::CreateVaultAccount;
use crate::errors::CustomError;
use anchor_lang::prelude::*;

pub fn create_vault(
    ctx: Context<CreateVaultAccount>,
    owners: Vec<Pubkey>,
    threshold: u8,
    _vault_id: u64,
    name: String,
) -> Result<()> {
    let mut unique_owners = std::collections::HashSet::new();
    for owner in &owners {
        require!(unique_owners.insert(*owner), CustomError::DuplicateOwner);
    }

    let vault = &mut ctx.accounts.vault;

    require!(
        threshold > 0 && threshold <= owners.len() as u8,
        CustomError::InvalidThreshold
    );
    require!(!owners.is_empty(), CustomError::EmptyOwners);
    require!(owners.len() <= 10, CustomError::TooManyOwners);
    require!(name.len() <= 20, CustomError::NameTooLong);
    require!(!name.trim().is_empty(), CustomError::EmptyName);

    vault.owners = owners;
    vault.threshold = threshold;
    vault.nonce = 0;
    vault.pending_transactions = Vec::new();
    vault.name = name;

    Ok(())
}
