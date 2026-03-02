# Sentel Contract

The Sentel program is a Solana smart contract that powers the Sentel protocol. It handles all on-chain vault logic including asset custody, multisig governance, token swaps via Jupiter, portfolio rebalancing, and full asset retrieval. The program supports two vault types: a standard multisig vault and a balanced vault with target token allocations.

## Content

This repository contains:

- The Sentel Anchor program.
- Tests for all instruction flows.

## Program Address

The Sentel program is deployed to:

- **Solana Mainnet-beta:** `Engn3cBYZPvP37myVuiwanqhs2omZxWRS7twNRJX8uZV`

## Build and test

You need Rust, the Solana CLI, and Anchor installed.

```bash
yarn install
anchor build
anchor test
```

To deploy to mainnet:

```bash
anchor deploy --provider.cluster mainnet
```

## Toolchain

```
anchor = "0.32.1"
solana  = "~2.1"
```

## Security

For responsible disclosure of security vulnerabilities, please refer to [SECURITY.md](SECURITY.md).

## License

The primary license for this repository is MIT, see [LICENSE](LICENSE).
