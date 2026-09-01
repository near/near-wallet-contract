# NEAR Wallet Contract

Ethereum-compatible wallet contract for NEAR Protocol, enabling Ethereum-style accounts and transactions on NEAR.

See [NEP-518](https://github.com/near/NEPs/issues/518) for the specification.

## Contracts

This repository contains two smart contracts:

- **wallet-contract** (`eth-wallet-contract`): The main Ethereum-compatible wallet contract that enables ETH-style transaction signing and execution on NEAR.
- **address-registrar** (`eth-address-registrar`): A registry contract that maps NEAR account IDs to Ethereum-style addresses (derived via keccak256 hashing).

## Building

### Prerequisites

- [Rust](https://rustup.rs/) 1.93.0+
- [cargo-near](https://github.com/near/cargo-near)

```bash
# Install cargo-near
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/near/cargo-near/releases/latest/download/cargo-near-installer.sh | sh
```

### Development Build

```bash
# Mainnet wallet
cargo near build non-reproducible-wasm --manifest-path wallet-contract/Cargo.toml

# Testnet wallet
cargo near build non-reproducible-wasm --manifest-path wallet-contract/Cargo.toml --features testnet

# Address registrar (the same artifact is used on both networks)
cargo near build non-reproducible-wasm --manifest-path address-registrar/Cargo.toml
```

### Reproducible Build (Production)

For production releases, use Docker-based reproducible builds per [NEP-0330](https://github.com/near/NEPs/blob/master/neps/nep-0330.md):

```bash
# Requires Docker
cargo near build reproducible-wasm --manifest-path wallet-contract/Cargo.toml --variant mainnet
cargo near build reproducible-wasm --manifest-path wallet-contract/Cargo.toml --variant testnet
cargo near build reproducible-wasm --manifest-path address-registrar/Cargo.toml
```

The reproducible build configuration is defined in each contract manifest under
`[package.metadata.near.reproducible_build]`.

## Testing

```bash
cargo test --workspace
```

## Releasing

Merging the release PR maintained by release-plz updates the workspace version,
`Cargo.lock`, and `CHANGELOG.md`, then creates the corresponding `vX.Y.Z` tag.
That tag triggers the existing release workflow, which builds and publishes the
reproducible WASM artifacts.

Publishing the released wallet WASMs as immutable global contracts is a separate
manual workflow with testnet-first and protected-mainnet gates. See
[Global wallet contract deployment](docs/global-contract-deployment.md) for the
environment setup and safety checks.

## Verification

Released WASM artifacts include:
- SHA256 checksums in `checksums.sha256`
- GitHub Actions attestations for build provenance

To verify a release:

```bash
# Download release artifacts
gh release download <version> -R near/near-wallet-contract

# Verify checksums
sha256sum -c checksums.sha256

# Verify attestation (when repo is public)
gh attestation verify eth_wallet_contract.wasm -R near/near-wallet-contract
```

## License

CC0-1.0
