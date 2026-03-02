use crate::contexts::GetVaultInfo;
use crate::VaultInfoEvent;
use anchor_lang::prelude::*;

pub fn get_vault_info(ctx: Context<GetVaultInfo>, _creator: Pubkey, _vault_id: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let actual_balance = vault.to_account_info().lamports();

    emit!(VaultInfoEvent {
        vault_address: ctx.accounts.vault.key(),
        owners: vault.owners.clone(),
        threshold: vault.threshold,
        balance: actual_balance,
        nonce: vault.nonce,
        pending_transactions_count: vault.pending_transactions.len() as u64,
        name: vault.name.clone(),
    });

    Ok(())
}
