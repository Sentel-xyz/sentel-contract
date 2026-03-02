use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Transfer};

use crate::{contexts::ExecuteTransaction, errors::CustomError};

pub fn execute_transaction(
    ctx: Context<ExecuteTransaction>,
    creator: Pubkey,
    vault_id: u64,
    _transaction_nonce: u64,
) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let transaction = &mut ctx.accounts.transaction;
    let signer = &ctx.accounts.signer;

    require!(
        vault.owners.contains(&signer.key()),
        CustomError::UnauthorizedProposer
    );

    require!(
        !transaction.executed,
        CustomError::TransactionAlreadyExecuted
    );

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp <= transaction.expires_at,
        CustomError::TransactionExpired
    );

    require!(
        transaction.approvals.len() >= vault.threshold as usize,
        CustomError::InsufficientApprovals
    );

    // Prevent self-transfers: target must not be the vault itself or the fee recipient.
    require!(
        transaction.target != vault.key(),
        CustomError::InvalidTarget
    );
    require!(
        transaction.target != ctx.accounts.fee_recipient.key(),
        CustomError::InvalidTarget
    );

    if transaction.token_type {
        let token_program = &ctx.accounts.token_program;
        let token_program_id = token_program.key();

        let token_2022_id = Pubkey::new_from_array([
            6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180,
            133, 237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
        ]);

        let is_valid_token_program =
            token_program_id == anchor_spl::token::ID || token_program_id == token_2022_id;

        require!(is_valid_token_program, CustomError::MissingTokenProgram);

        // Deserialize token accounts
        let vault_token_account = TokenAccount::try_deserialize(
            &mut &ctx.accounts.vault_token_account.try_borrow_data()?[..],
        )?;
        let target_token_account = TokenAccount::try_deserialize(
            &mut &ctx.accounts.target_token_account.try_borrow_data()?[..],
        )?;

        // Verify token accounts match the transaction mint
        require!(
            vault_token_account.mint == transaction.mint,
            CustomError::InvalidMint
        );
        require!(
            target_token_account.mint == transaction.mint,
            CustomError::InvalidMint
        );

        // Verify vault owns the source token account
        require!(
            vault_token_account.owner == vault.key(),
            CustomError::InvalidTokenAccount
        );

        // For SPL tokens, charge only the minimum fee (flat rate)
        let fee_amount = if transaction.amount > 0 {
            crate::MIN_FEE_LAMPORTS
        } else {
            0
        };

        let vault_sol_balance = vault.to_account_info().lamports();
        require!(
            vault_sol_balance >= fee_amount,
            CustomError::InsufficientFundsForFee
        );

        // Transfer tokens from vault to target
        let vault_id_bytes = vault_id.to_le_bytes();
        let seeds = &[
            b"vault".as_ref(),
            creator.as_ref(),
            vault_id_bytes.as_ref(),
            &[ctx.bumps.vault],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.target_token_account.to_account_info(),
            authority: vault.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        token::transfer(cpi_ctx, transaction.amount)?;

        // Collect fee in SOL if applicable
        if fee_amount > 0 {
            let vault_info = vault.to_account_info();
            let fee_recipient_info = ctx.accounts.fee_recipient.to_account_info();

            **vault_info.try_borrow_mut_lamports()? = vault_sol_balance
                .checked_sub(fee_amount)
                .ok_or(CustomError::InsufficientFunds)?;
            **fee_recipient_info.try_borrow_mut_lamports()? = fee_recipient_info
                .lamports()
                .checked_add(fee_amount)
                .ok_or(CustomError::InsufficientFunds)?;
        }
    } else {
        // SOL Transfer (existing logic)
        let vault_balance_before = vault.to_account_info().lamports();

        let calculated_fee = transaction
            .amount
            .checked_mul(crate::PROTOCOL_FEE_BASIS_POINTS)
            .and_then(|x| x.checked_div(10000))
            .unwrap_or(0);

        let fee_amount = if transaction.amount > 0 {
            calculated_fee.min(crate::MAX_FEE_LAMPORTS) // no floor  proportional only
        } else {
            0
        };

        let total_deduction = transaction
            .amount
            .checked_add(fee_amount)
            .ok_or(CustomError::InsufficientFunds)?;

        require!(
            vault_balance_before >= total_deduction,
            CustomError::InsufficientFunds
        );

        let vault_info = vault.to_account_info();
        let target_info = ctx.accounts.target.to_account_info();
        let fee_recipient_info = ctx.accounts.fee_recipient.to_account_info();

        let vault_rent_exempt_minimum = Rent::get()?.minimum_balance(vault_info.data_len());
        let remaining_after_transfer = vault_balance_before.saturating_sub(total_deduction);

        require!(
            remaining_after_transfer >= vault_rent_exempt_minimum,
            CustomError::InsufficientFunds
        );

        let vault_lamports = vault_info.lamports();
        let target_lamports = target_info.lamports();
        let fee_recipient_lamports = fee_recipient_info.lamports();

        require!(
            target_lamports.checked_add(transaction.amount).is_some(),
            CustomError::InsufficientFunds
        );
        require!(
            fee_recipient_lamports.checked_add(fee_amount).is_some(),
            CustomError::InsufficientFunds
        );

        // Deduct amount + fee from vault in one atomic operation using the snapshot.
        **vault_info.try_borrow_mut_lamports()? = vault_lamports
            .checked_sub(total_deduction)
            .ok_or(CustomError::InsufficientFunds)?;
        **target_info.try_borrow_mut_lamports()? = target_lamports
            .checked_add(transaction.amount)
            .ok_or(CustomError::InsufficientFunds)?;

        if fee_amount > 0 {
            **fee_recipient_info.try_borrow_mut_lamports()? = fee_recipient_lamports
                .checked_add(fee_amount)
                .ok_or(CustomError::InsufficientFunds)?;
        }
    }

    transaction.executed = true;

    vault
        .pending_transactions
        .retain(|&id| id != transaction.id);

    Ok(())
}
