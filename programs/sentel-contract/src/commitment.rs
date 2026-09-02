use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

/// Binds an approved proposal to the exact CPI it authorises.
///
/// Owners approve a proposal, but the Jupiter payload and its account list are
/// supplied by whoever executes it. Without a commitment the approval is a blank
/// cheque: the executor can substitute a different route, a different source
/// account, or a destination they control. Hashing the payload together with the
/// ordered account keys at propose time, and re-deriving it at execute time,
/// makes the approval mean the thing the owners actually looked at.
///
/// The account keys are included in order because Jupiter reads the source and
/// destination token accounts positionally from the account list, not from the
/// payload.
pub fn swap_commitment(data: &[u8], accounts: &[AccountInfo]) -> [u8; 32] {
    let keys: Vec<[u8; 32]> = accounts.iter().map(|acc| acc.key.to_bytes()).collect();

    let mut parts: Vec<&[u8]> = Vec::with_capacity(keys.len() + 1);
    parts.push(data);
    for key in &keys {
        parts.push(key.as_ref());
    }

    hashv(&parts).to_bytes()
}
