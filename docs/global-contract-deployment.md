# Global wallet contract deployment

The manual **Deploy Global Wallet Contracts** workflow publishes the released
testnet WASM first, then waits for the protected `mainnet` environment before
publishing the mainnet WASM. The address registrar is deployed separately.

## GitHub setup

Create `testnet` and `mainnet` environments under **Settings → Environments**.
Configure each with:

- Secret `NEAR_SIGNER_PRIVATE_KEY`: the FullAccess ed25519 key for a dedicated,
  low-balance publisher account.
- Variable `NEAR_SIGNER_ACCOUNT_ID`: the publisher account ID.
- Variable `NEAR_RPC_URL`: a trusted RPC URL for that network.
- A deployment branch policy limited to `main`.

For `mainnet`, require a reviewer, prevent self-review, and disable administrator
bypass. Finance can fund the publisher separately and just in time; treasury
credentials never belong in GitHub.

Run the workflow from `main` with an immutable stable release tag. It verifies
the release, checksum, and GitHub attestations, then calls
`deploy-global-contract-network.yml` once per network. That reusable workflow
computes the global contract hash (base58 of the SHA256 in `checksums.sha256`),
skips the deployment if that hash already resolves to identical bytes on-chain,
otherwise deploys with near-cli-rs and waits for finality, and finally
downloads the global contract by hash and compares it byte-for-byte with the
release artifact. The non-cancelling concurrency group serializes runs.

The skip matters: redeploying an existing global contract succeeds and burns
the full storage fee again (about 25 NEAR for the current wallet WASM).

## Manual deployment

The deployment is three near-cli commands. To run it by hand, verify the
release asset as the workflow does, then run the `Compute global contract
hash`, `Deploy`, and `Verify on-chain bytes` steps from
`.github/workflows/deploy-global-contract-network.yml` with `WASM`, `NETWORK`,
`SIGNER_ACCOUNT`, and `SIGNER_KEY` set. Against a local
`nearprotocol/sandbox` container, add a `[network_connection.sandbox]` block to
near-cli's `config.toml` and use `sandbox` as both network and signer.
