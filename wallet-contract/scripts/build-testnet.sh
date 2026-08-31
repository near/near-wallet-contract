#!/bin/sh
set -eu

# Chain ID 398 - https://chainlist.org/chain/398
printf '{ 398 }\n' > src/CHAIN_ID
printf 'address-map.testnet' > src/ADDRESS_REGISTRAR_ACCOUNT_ID

exec cargo near build non-reproducible-wasm --locked --no-abi
