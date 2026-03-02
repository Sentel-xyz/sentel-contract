use crate::contexts::ExecuteRetrieveTransaction;
use crate::errors::CustomError;
use crate::instructions::jupiter_account_meta;
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{instruction::Instruction, program::invoke_signed};
use anchor_spl::token::{self, CloseAccount};
use std::str::FromStr;

pub fn execute_retrieve_transaction<'info>(
    ctx: Context<'_, '_, '_, 'info, ExecuteRetrieveTransaction<'info>>,
    vault_id: u64,
    _transaction_nonce: u64,
    jupiter_swap_data: Vec<Vec<u8>>,
    swap_account_counts: Vec<u32>,
) -> Result<()> {
    // Read all values we need before taking mutable borrows
    let executor_key = ctx.accounts.executor.key();
    let is_owner = ctx.accounts.balanced_vault.owners.contains(&executor_key);
    let is_active = ctx.accounts.balanced_vault.is_active;
    let already_executed = ctx.accounts.retrieve_transaction.executed;
    let expires_at = ctx.accounts.retrieve_transaction.expires_at;
    let num_approvals = ctx.accounts.retrieve_transaction.approvals.len();
    let threshold = ctx.accounts.balanced_vault.threshold as usize;
    let recipient_key = ctx.accounts.retrieve_transaction.recipient;
    let tx_id = ctx.accounts.retrieve_transaction.id;
    let creator_key = ctx.accounts.balanced_vault.creator;
    let vault_bump = ctx.accounts.balanced_vault.bump;

    require!(is_owner, CustomError::Unauthorized);
    require!(is_active, CustomError::BalancedVaultNotActive);
    require!(!already_executed, CustomError::TransactionAlreadyExecuted);

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < expires_at,
        CustomError::TransactionExpired
    );
    require!(
        num_approvals >= threshold,
        CustomError::InsufficientApprovals
    );

    // Verify the recipient account matches what was proposed
    require!(
        ctx.accounts.recipient.key() == recipient_key,
        CustomError::Unauthorized
    );

    let vault_id_bytes = vault_id.to_le_bytes();
    let seeds = &[
        b"balanced_vault".as_ref(),
        creator_key.as_ref(),
        vault_id_bytes.as_ref(),
        &[vault_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    // Step 1: Execute Jupiter swaps to convert all tokens -> WSOL
    if !jupiter_swap_data.is_empty() {
        let jupiter_program_id = Pubkey::from_str(crate::JUPITER_V6_PROGRAM_ID)
            .map_err(|_| CustomError::InvalidFeeRecipient)?;

        require!(
            ctx.accounts.jupiter_program.key() == jupiter_program_id,
            CustomError::InvalidFeeRecipient
        );

        // L-3 fix: ensure swap_account_counts is aligned with swap data to prevent
        // out-of-bounds slicing and silent account misallocation.
        require!(
            swap_account_counts.len() == jupiter_swap_data.len(),
            CustomError::InvalidAmount
        );

        let balanced_vault_key = ctx.accounts.balanced_vault.key();
        let remaining_accounts = ctx.remaining_accounts;
        let mut account_offset: usize = 0;

        for (i, swap_data) in jupiter_swap_data.iter().enumerate() {
            let account_count = swap_account_counts[i] as usize;

            let swap_account_infos =
                &remaining_accounts[account_offset..account_offset + account_count];

            let swap_accounts: Vec<_> = swap_account_infos
                .iter()
                .map(|acc| jupiter_account_meta(acc, &balanced_vault_key))
                .collect();

            let jupiter_ix = Instruction {
                program_id: jupiter_program_id,
                accounts: swap_accounts,
                data: swap_data.clone(),
            };

            invoke_signed(&jupiter_ix, swap_account_infos, signer_seeds)?;

            account_offset += account_count;
        }

        // All tokens converted to WSOL; proceed to unwrap.
    } else {
        // No token swaps needed; proceed directly to WSOL unwrap.
    }

    // Step 2: Unwrap WSOL -> vault PDA (as native SOL), then distribute fee + net to recipient.
    //
    // We close the WSOL token account into the vault PDA (not directly into recipient) so that
    // we control the SOL. Then we use direct lamport manipulation  which is legal because the
    // vault PDA is owned by this program  to send fee to fee_recipient and the rest to recipient.
    // This avoids Anchor's post-instruction lamport conservation error that occurs when you mix
    // direct lamport changes on Anchor-tracked accounts with subsequent CPIs.
    let wsol_lamports = ctx.accounts.vault_wsol_account.to_account_info().lamports();
    let wsol_rent = Rent::get()?.minimum_balance(165); // SPL token account = 165 bytes
    let effective_wsol = wsol_lamports.saturating_sub(wsol_rent);

    let fee_bps = crate::PROTOCOL_FEE_BASIS_POINTS; // 5 bps = 0.05%
    let raw_fee = effective_wsol
        .saturating_mul(fee_bps)
        .checked_div(10_000)
        .unwrap_or(0);
    let fee_amount = raw_fee
        .max(crate::MIN_FEE_LAMPORTS)
        .min(crate::MAX_FEE_LAMPORTS);

    // CPI first: close WSOL token account, sending all lamports into the vault PDA.
    // After this CPI completes, no more CPIs  only direct lamport manipulation.
    let cpi_accounts = CloseAccount {
        account: ctx.accounts.vault_wsol_account.to_account_info(),
        destination: ctx.accounts.balanced_vault.to_account_info(),
        authority: ctx.accounts.balanced_vault.to_account_info(),
    };

    token::close_account(CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    ))?;

    // Now vault PDA lamports = original vault lamports + wsol_lamports.
    // Distribute: fee -> fee_recipient, remainder -> recipient.
    // Direct lamport manipulation is safe here: vault PDA is owned by this program,
    // and we only ADD to fee_recipient and recipient (always allowed for any account).
    {
        let vault_info = ctx.accounts.balanced_vault.to_account_info();
        let vault_post = vault_info.lamports();
        let fee_info = ctx.accounts.fee_recipient.to_account_info();
        let recipient_info = ctx.accounts.recipient.to_account_info();

        // M-3: Fee is calculated on effective_wsol (excluding rent), so net to recipient is
        // also based on effective_wsol. The rent portion stays in the vault (returned naturally
        // when the token account is closed back into the vault PDA).
        let net_to_recipient = effective_wsol
            .checked_sub(fee_amount)
            .ok_or(CustomError::InsufficientFunds)?;

        // vault_post = pre-close vault + wsol_lamports (rent included).
        // We send fee + net out; the remainder (wsol_rent) stays in the vault.
        **vault_info.try_borrow_mut_lamports()? = vault_post
            .checked_sub(net_to_recipient)
            .ok_or(CustomError::InsufficientFunds)?
            .checked_sub(fee_amount)
            .ok_or(CustomError::InsufficientFunds)?;
        **fee_info.try_borrow_mut_lamports()? = fee_info
            .lamports()
            .checked_add(fee_amount)
            .ok_or(CustomError::InsufficientFunds)?;
        **recipient_info.try_borrow_mut_lamports()? = recipient_info
            .lamports()
            .checked_add(net_to_recipient)
            .ok_or(CustomError::InsufficientFunds)?;
    }

    // Mark executed and remove from pending
    ctx.accounts.retrieve_transaction.executed = true;
    ctx.accounts
        .balanced_vault
        .pending_transactions
        .retain(|&id| id != tx_id);

    Ok(())
}
