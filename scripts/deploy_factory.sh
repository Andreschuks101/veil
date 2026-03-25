#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Prerequisites:
#   - Rust with the wasm32-unknown-unknown target:
#       rustup target add wasm32-unknown-unknown
#   - Stellar CLI (https://developers.stellar.org/docs/tools/developer-tools/cli/install-stellar-cli):
#       cargo install --locked stellar-cli --features opt
#   - A configured Deployer key alias in the Stellar CLI:
#       stellar keys generate --global <alias> --network testnet
#
# Usage:
#   ./scripts/deploy_factory.sh [--source <key-alias>] [--write-env]
#
#   --source <alias>   Stellar CLI key alias for the deployer account.
#                      Falls back to the $DEPLOYER environment variable.
#   --write-env        If set, upserts FACTORY_CONTRACT_ID in .env.testnet.
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DEPLOYER_ALIAS="${DEPLOYER:-}"
WRITE_ENV=false

usage() {
  echo "Usage: $0 [--source <key-alias>] [--write-env]" >&2
  echo "" >&2
  echo "  --source <alias>   Stellar CLI key alias for the deployer account." >&2
  echo "                     Falls back to the \$DEPLOYER environment variable." >&2
  echo "  --write-env        Upsert FACTORY_CONTRACT_ID in .env.testnet." >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source)
      if [[ $# -lt 2 || -z "$2" ]]; then
        echo "Error: --source requires a non-empty key alias argument." >&2
        usage
        exit 1
      fi
      DEPLOYER_ALIAS="$2"
      shift 2
      ;;
    --write-env)
      WRITE_ENV=true
      shift
      ;;
    *)
      echo "Error: Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$DEPLOYER_ALIAS" ]]; then
  echo "Error: No deployer key alias provided. Supply --source <alias> or set the \$DEPLOYER environment variable." >&2
  usage
  exit 1
fi

# --- Phase 2: Build ---
echo "Building factory contract Wasm..."
cd "$PROJECT_ROOT/contracts/invisible_wallet"
cargo build --target wasm32-unknown-unknown --release || {
  echo "Error: cargo build failed." >&2
  exit 1
}
cd "$PROJECT_ROOT"

# --- Phase 3: Upload Wasm ---
WASM_PATH="$PROJECT_ROOT/contracts/invisible_wallet/target/wasm32-unknown-unknown/release/invisible_wallet.wasm"
echo "Uploading Wasm to testnet..."
UPLOAD_EXIT=0
WASM_HASH=$(stellar contract upload \
  --network testnet \
  --source "$DEPLOYER_ALIAS" \
  --wasm "$WASM_PATH") || UPLOAD_EXIT=$?

if [[ $UPLOAD_EXIT -ne 0 ]]; then
  echo "Error: Wasm upload failed with exit code $UPLOAD_EXIT." >&2
  exit 1
fi

if [[ -z "$WASM_HASH" ]]; then
  echo "Error: Upload returned empty hash." >&2
  exit 1
fi

echo "Wasm hash: $WASM_HASH"

# --- Phase 4: Deploy Contract ---
echo "Deploying factory contract to testnet..."
DEPLOY_EXIT=0
CONTRACT_ID=$(stellar contract deploy \
  --network testnet \
  --source "$DEPLOYER_ALIAS" \
  --wasm-hash "$WASM_HASH") || DEPLOY_EXIT=$?

if [[ $DEPLOY_EXIT -ne 0 ]]; then
  echo "Error: Contract deploy failed with exit code $DEPLOY_EXIT." >&2
  exit 1
fi

if [[ -z "$CONTRACT_ID" ]]; then
  echo "Error: Deploy returned empty contract ID." >&2
  exit 1
fi

# --- Phase 6: Write env file (optional) ---
if [[ "$WRITE_ENV" == "true" ]]; then
  ENV_FILE="$PROJECT_ROOT/.env.testnet"
  if grep -q "^FACTORY_CONTRACT_ID=" "$ENV_FILE" 2>/dev/null; then
    sed -i.bak "s|^FACTORY_CONTRACT_ID=.*|FACTORY_CONTRACT_ID=$CONTRACT_ID|" "$ENV_FILE"
    rm -f "$ENV_FILE.bak"
  else
    echo "FACTORY_CONTRACT_ID=$CONTRACT_ID" >> "$ENV_FILE"
  fi
  echo "Written FACTORY_CONTRACT_ID to $ENV_FILE" >&2
fi

# --- Phase 5: Output ---
echo "Factory Contract ID: $CONTRACT_ID"
