use crate::contexts::CloseVault;
use crate::errors::CustomError;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, TokenAccount};

pub fn close_vault<'info>(
    ctx: Context<'_, '_, '_, 'info, CloseVault<'info>>,
    creator_key: Pubkey,
    vault_id: u64,
) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let creator_signer = &ctx.accounts.creator_signer;

    // Check that there are no pending transactions
    require!(
        vault.pending_transactions.is_empty(),
        CustomError::VaultHasPendingTransactions
    );

    // Close all provided token accounts
    let vault_key = vault.key();
    let vault_bump = ctx.bumps.vault;
    let vault_seeds = &[
        b"vault".as_ref(),
        creator_key.as_ref(),
        &vault_id.to_le_bytes(),
        &[vault_bump],
    ];
    let signer_seeds = &[&vault_seeds[..]];

    for account_info in ctx.remaining_accounts.iter() {
        // Only process accounts that are owned by the SPL Token program to prevent
        // passing arbitrary accounts (which would fail deserialization but waste compute).
        if account_info.owner != &anchor_spl::token::ID {
            continue;
        }

        let should_close = {
            if let Ok(token_account) =
                TokenAccount::try_deserialize(&mut &account_info.data.borrow()[..])
            {
                if token_account.owner == vault_key {
                    require!(token_account.amount == 0, CustomError::VaultHasTokenBalance);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_close {
            let close_account_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                CloseAccount {
                    account: account_info.clone(),
                    destination: creator_signer.to_account_info(),
                    authority: vault.to_account_info(),
                },
                signer_seeds,
            );
            token::close_account(close_account_ctx)?;
        }
    }

    let vault_info = vault.to_account_info();
    let current_balance = vault_info.lamports();
    const MAX_SOL_FOR_CLOSURE: u64 = 300_000_000; // 0.3 SOL

    require!(
        current_balance < MAX_SOL_FOR_CLOSURE,
        CustomError::VaultBalanceTooHighForClosure
    );

    let clock = Clock::get()?;
    emit!(crate::VaultClosedEvent {
        vault_address: vault.key(),
        creator: creator_key,
        final_balance: current_balance,
        timestamp: clock.unix_timestamp,
    });

    if current_balance > 0 {
        let creator_info = creator_signer.to_account_info();
        let creator_balance = creator_info.lamports();

        require!(
            creator_balance.checked_add(current_balance).is_some(),
            CustomError::InsufficientFunds
        );

        **vault_info.try_borrow_mut_lamports()? = 0;
        **creator_info.try_borrow_mut_lamports()? = creator_balance
            .checked_add(current_balance)
            .ok_or(CustomError::InsufficientFunds)?;
    }

    Ok(())
}
