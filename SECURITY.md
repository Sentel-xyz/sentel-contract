# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in this program, **do not open a public GitHub issue**.

Report it privately by emailing: **sentel_xyz@proton.me**

Please include:

- A description of the vulnerability
- Steps to reproduce or a proof-of-concept
- The potential impact
- Your suggested fix (optional)

We will acknowledge your report within 48 hours and aim to release a patch within 7 days for critical issues.

## Scope

The following is in scope:

- `programs/sentel-contract/src/`  -  all on-chain Rust/Anchor code
- Any logic that could result in loss of user funds or bypass of the multisig threshold

The following is out of scope:

- The frontend at `sentel.xyz` (UI bugs, not fund loss)
- Denial-of-service via transaction spam
- Issues requiring a compromised owner keypair

## Program

| Network | Program ID                                     |
| ------- | ---------------------------------------------- |
| Mainnet | `Engn3cBYZPvP37myVuiwanqhs2omZxWRS7twNRJX8uZV` |
