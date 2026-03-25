#!/usr/bin/env bats
# Tests for scripts/deploy_factory.sh
# Uses stub binaries placed in a temp PATH to avoid real network calls.

SCRIPT="$BATS_TEST_DIRNAME/../deploy_factory.sh"

setup() {
  STUB_DIR="$(mktemp -d)"
  export PATH="$STUB_DIR:$PATH"

  # Default stub: cargo succeeds
  cat > "$STUB_DIR/cargo" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$STUB_DIR/cargo"

  # Default stub: stellar upload returns a fixed hash, deploy returns a fixed ID
  cat > "$STUB_DIR/stellar" <<'EOF'
#!/usr/bin/env bash
if [[ "$1 $2" == "contract upload" ]]; then
  echo "aabbccdd1122334455667788990011223344556677889900aabbccdd11223344"
elif [[ "$1 $2" == "contract deploy" ]]; then
  echo "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
fi
exit 0
EOF
  chmod +x "$STUB_DIR/stellar"

  WORK_DIR="$(mktemp -d)"
  export BATS_TMPDIR="$WORK_DIR"
}

teardown() {
  rm -rf "$STUB_DIR" "$WORK_DIR"
}

# ---------------------------------------------------------------------------
# E1 — Prerequisite comment block
# ---------------------------------------------------------------------------
@test "E1: script contains prerequisite comment block" {
  grep -q "wasm32-unknown-unknown" "$SCRIPT"
  grep -q "Stellar CLI" "$SCRIPT"
  grep -q "\-\-source" "$SCRIPT"
  grep -q "Usage:" "$SCRIPT"
}

# ---------------------------------------------------------------------------
# E2 — cargo build is called with correct args
# ---------------------------------------------------------------------------
@test "E2: cargo build is called with --target wasm32-unknown-unknown --release" {
  LOG="$STUB_DIR/cargo.log"
  cat > "$STUB_DIR/cargo" <<EOF
#!/usr/bin/env bash
echo "CARGO_ARGS: \$*" >> "$LOG"
exit 0
EOF
  chmod +x "$STUB_DIR/cargo"

  run bash "$SCRIPT" --source myalias
  [ "$status" -eq 0 ]
  grep -q "wasm32-unknown-unknown" "$LOG"
  grep -q "release" "$LOG"
}

# ---------------------------------------------------------------------------
# E3 — stellar contract upload receives correct flags
# ---------------------------------------------------------------------------
@test "E3: stellar upload receives --network testnet --source and --wasm" {
  LOG="$STUB_DIR/stellar.log"
  cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
echo "STELLAR_ARGS: \$*" >> "$LOG"
if [[ "\$1 \$2" == "contract upload" ]]; then
  echo "aabbccdd1122334455667788990011223344556677889900aabbccdd11223344"
elif [[ "\$1 \$2" == "contract deploy" ]]; then
  echo "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
fi
exit 0
EOF
  chmod +x "$STUB_DIR/stellar"

  run bash "$SCRIPT" --source myalias
  [ "$status" -eq 0 ]
  grep -q "\-\-network testnet" "$LOG"
  grep -q "\-\-source myalias" "$LOG"
  grep -q "\-\-wasm" "$LOG"
}

# ---------------------------------------------------------------------------
# E4 — Wasm hash from upload is passed to deploy (idempotency)
# ---------------------------------------------------------------------------
@test "E4: wasm hash from upload is passed to stellar deploy" {
  FIXED_HASH="deadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678"
  cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
echo "STELLAR_ARGS: \$*" >> "$STUB_DIR/stellar.log"
if [[ "\$1 \$2" == "contract upload" ]]; then
  echo "$FIXED_HASH"
elif [[ "\$1 \$2" == "contract deploy" ]]; then
  echo "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
fi
exit 0
EOF
  chmod +x "$STUB_DIR/stellar"

  run bash "$SCRIPT" --source myalias
  [ "$status" -eq 0 ]
  grep -q "\-\-wasm-hash $FIXED_HASH" "$STUB_DIR/stellar.log"
}

# ---------------------------------------------------------------------------
# E5 — stellar deploy receives correct flags
# ---------------------------------------------------------------------------
@test "E5: stellar deploy receives --network testnet --source and --wasm-hash" {
  LOG="$STUB_DIR/stellar.log"
  cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
echo "STELLAR_ARGS: \$*" >> "$LOG"
if [[ "\$1 \$2" == "contract upload" ]]; then
  echo "aabbccdd1122334455667788990011223344556677889900aabbccdd11223344"
elif [[ "\$1 \$2" == "contract deploy" ]]; then
  echo "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
fi
exit 0
EOF
  chmod +x "$STUB_DIR/stellar"

  run bash "$SCRIPT" --source myalias
  [ "$status" -eq 0 ]
  grep -q "\-\-network testnet" "$LOG"
  grep -q "\-\-source myalias" "$LOG"
  grep -q "\-\-wasm-hash" "$LOG"
}

# ---------------------------------------------------------------------------
# E6 — deployer alias resolution: --source arg and $DEPLOYER env var
# ---------------------------------------------------------------------------
@test "E6a: --source arg is passed to stellar" {
  LOG="$STUB_DIR/stellar.log"
  cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
echo "STELLAR_ARGS: \$*" >> "$LOG"
if [[ "\$1 \$2" == "contract upload" ]]; then echo "hash123"; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then echo "CCONTRACTID"; fi
exit 0
EOF
  chmod +x "$STUB_DIR/stellar"

  run bash "$SCRIPT" --source cli-alias
  [ "$status" -eq 0 ]
  grep -q "\-\-source cli-alias" "$LOG"
}

@test "E6b: DEPLOYER env var is used when --source is not supplied" {
  LOG="$STUB_DIR/stellar.log"
  cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
echo "STELLAR_ARGS: \$*" >> "$LOG"
if [[ "\$1 \$2" == "contract upload" ]]; then echo "hash123"; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then echo "CCONTRACTID"; fi
exit 0
EOF
  chmod +x "$STUB_DIR/stellar"

  DEPLOYER=env-alias run bash "$SCRIPT"
  [ "$status" -eq 0 ]
  grep -q "\-\-source env-alias" "$LOG"
}

# ---------------------------------------------------------------------------
# Property 1 — failure exits non-zero with stderr (build failure)
# # Feature: deploy-factory-script, Property 1: Failure conditions exit non-zero with stderr output
# ---------------------------------------------------------------------------
@test "Property 1: cargo build failure exits non-zero with stderr" {
  for i in $(seq 1 20); do
    EXIT_CODE=$(( (RANDOM % 127) + 1 ))
    cat > "$STUB_DIR/cargo" <<EOF
#!/usr/bin/env bash
exit $EXIT_CODE
EOF
    chmod +x "$STUB_DIR/cargo"
    run bash "$SCRIPT" --source myalias
    [ "$status" -ne 0 ]
    [ -n "$output" ]
  done
}

# ---------------------------------------------------------------------------
# Property 1 — upload failure
# # Feature: deploy-factory-script, Property 1: Failure conditions exit non-zero with stderr output
# ---------------------------------------------------------------------------
@test "Property 1: stellar upload failure exits non-zero with stderr" {
  for i in $(seq 1 20); do
    EXIT_CODE=$(( (RANDOM % 127) + 1 ))
    cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
if [[ "\$1 \$2" == "contract upload" ]]; then exit $EXIT_CODE; fi
exit 0
EOF
    chmod +x "$STUB_DIR/stellar"
    run bash "$SCRIPT" --source myalias
    [ "$status" -ne 0 ]
    [ -n "$output" ]
  done
}

# ---------------------------------------------------------------------------
# Property 1 — deploy failure
# # Feature: deploy-factory-script, Property 1: Failure conditions exit non-zero with stderr output
# ---------------------------------------------------------------------------
@test "Property 1: stellar deploy failure exits non-zero with stderr" {
  for i in $(seq 1 20); do
    EXIT_CODE=$(( (RANDOM % 127) + 1 ))
    cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
if [[ "\$1 \$2" == "contract upload" ]]; then echo "somehash"; exit 0; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then exit $EXIT_CODE; fi
exit 0
EOF
    chmod +x "$STUB_DIR/stellar"
    run bash "$SCRIPT" --source myalias
    [ "$status" -ne 0 ]
    [ -n "$output" ]
  done
}

# ---------------------------------------------------------------------------
# Property 1 — missing deployer
# # Feature: deploy-factory-script, Property 1: Failure conditions exit non-zero with stderr output
# ---------------------------------------------------------------------------
@test "Property 1: missing deployer alias exits non-zero with stderr" {
  for i in $(seq 1 20); do
    unset DEPLOYER || true
    run bash "$SCRIPT"
    [ "$status" -ne 0 ]
    [ -n "$output" ]
  done
}

# ---------------------------------------------------------------------------
# Property 2 — deployer identity via key alias only (no raw secret keys)
# # Feature: deploy-factory-script, Property 2: Deployer identity is always passed via key alias
# ---------------------------------------------------------------------------
@test "Property 2: stellar invocations use --source alias, never raw secret key" {
  for i in $(seq 1 20); do
    ALIAS="alias${RANDOM}x${i}"
    LOG="$STUB_DIR/stellar_p2_${i}.log"
    cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
echo "STELLAR_ARGS: \$*" >> "$LOG"
if [[ "\$1 \$2" == "contract upload" ]]; then echo "hash$i"; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then echo "CCONTRACT$i"; fi
exit 0
EOF
    chmod +x "$STUB_DIR/stellar"

    run bash "$SCRIPT" --source "$ALIAS"
    [ "$status" -eq 0 ]
    grep -q "\-\-source $ALIAS" "$LOG"
    # Assert no raw Stellar secret key pattern (S followed by 55 base32 chars)
    run grep -P "S[A-Z2-7]{55}" "$LOG"
    [ "$status" -ne 0 ]
  done
}

# ---------------------------------------------------------------------------
# Property 3 — Contract ID is the final labeled stdout line
# # Feature: deploy-factory-script, Property 3: Contract ID output is the final labeled stdout line
# ---------------------------------------------------------------------------
@test "Property 3: final stdout line is 'Factory Contract ID: <id>'" {
  for i in $(seq 1 20); do
    CONTRACT="CTEST${RANDOM}X${i}"
    cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
if [[ "\$1 \$2" == "contract upload" ]]; then echo "hash$i"; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then echo "$CONTRACT"; fi
exit 0
EOF
    chmod +x "$STUB_DIR/stellar"

    run bash "$SCRIPT" --source myalias
    [ "$status" -eq 0 ]
    LAST_LINE="${lines[${#lines[@]}-1]}"
    [ "$LAST_LINE" = "Factory Contract ID: $CONTRACT" ]
  done
}

# ---------------------------------------------------------------------------
# Property 4 — --write-env writes FACTORY_CONTRACT_ID to .env.testnet
# # Feature: deploy-factory-script, Property 4: --write-env writes FACTORY_CONTRACT_ID to .env.testnet
# ---------------------------------------------------------------------------
@test "Property 4: --write-env writes FACTORY_CONTRACT_ID to .env.testnet" {
  for i in $(seq 1 5); do
    CONTRACT="CTEST$i"
    cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
if [[ "\$1 \$2" == "contract upload" ]]; then echo "hash$i"; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then echo "$CONTRACT"; fi
exit 0
EOF
    chmod +x "$STUB_DIR/stellar"
    rm -f "$WORK_DIR/.env.testnet"

    # Run from WORK_DIR so .env.testnet lands there; script uses PROJECT_ROOT
    # We override by running from a temp dir and pointing script root there
    SCRIPT_COPY="$WORK_DIR/scripts/deploy_factory.sh"
    mkdir -p "$WORK_DIR/scripts" "$WORK_DIR/contracts/invisible_wallet"
    cp "$SCRIPT" "$SCRIPT_COPY"

    run bash "$SCRIPT_COPY" --source myalias --write-env
    [ "$status" -eq 0 ]
    grep -q "FACTORY_CONTRACT_ID=$CONTRACT" "$WORK_DIR/.env.testnet"
    rm -rf "$WORK_DIR/scripts" "$WORK_DIR/contracts" "$WORK_DIR/.env.testnet"
  done
}

# ---------------------------------------------------------------------------
# Property 5 — --write-env preserves other lines in .env.testnet
# # Feature: deploy-factory-script, Property 5: --write-env preserves all other lines in .env.testnet
# ---------------------------------------------------------------------------
@test "Property 5: --write-env preserves existing lines in .env.testnet" {
  for i in $(seq 1 5); do
    CONTRACT="CTEST$i"
    cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
if [[ "\$1 \$2" == "contract upload" ]]; then echo "hash$i"; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then echo "$CONTRACT"; fi
exit 0
EOF
    chmod +x "$STUB_DIR/stellar"

    SCRIPT_COPY="$WORK_DIR/scripts/deploy_factory.sh"
    mkdir -p "$WORK_DIR/scripts" "$WORK_DIR/contracts/invisible_wallet"
    cp "$SCRIPT" "$SCRIPT_COPY"

    # Pre-populate .env.testnet with other lines
    printf "OTHER_VAR=hello\nANOTHER=world\n" > "$WORK_DIR/.env.testnet"

    run bash "$SCRIPT_COPY" --source myalias --write-env
    [ "$status" -eq 0 ]
    grep -q "OTHER_VAR=hello" "$WORK_DIR/.env.testnet"
    grep -q "ANOTHER=world" "$WORK_DIR/.env.testnet"
    grep -q "FACTORY_CONTRACT_ID=$CONTRACT" "$WORK_DIR/.env.testnet"
    # Clean up for next iteration
    rm -rf "$WORK_DIR/scripts" "$WORK_DIR/contracts" "$WORK_DIR/.env.testnet"
  done
}

# ---------------------------------------------------------------------------
# Property 6 — without --write-env, .env.testnet is not created or modified
# # Feature: deploy-factory-script, Property 6: Without --write-env, .env.testnet is not created or modified
# ---------------------------------------------------------------------------
@test "Property 6: without --write-env, .env.testnet is not created" {
  for i in $(seq 1 5); do
    SCRIPT_COPY="$WORK_DIR/scripts/deploy_factory.sh"
    mkdir -p "$WORK_DIR/scripts" "$WORK_DIR/contracts/invisible_wallet"
    cp "$SCRIPT" "$SCRIPT_COPY"
    rm -f "$WORK_DIR/.env.testnet"

    run bash "$SCRIPT_COPY" --source myalias
    [ "$status" -eq 0 ]
    [ ! -f "$WORK_DIR/.env.testnet" ]
    rm -rf "$WORK_DIR/scripts" "$WORK_DIR/contracts"
  done
}

# ---------------------------------------------------------------------------
# Property 7 — successful deployment always exits 0
# # Feature: deploy-factory-script, Property 7: Successful deployment always exits 0
# ---------------------------------------------------------------------------
@test "Property 7: successful deployment exits 0" {
  for i in $(seq 1 20); do
    cat > "$STUB_DIR/stellar" <<EOF
#!/usr/bin/env bash
if [[ "\$1 \$2" == "contract upload" ]]; then echo "hash$i"; fi
if [[ "\$1 \$2" == "contract deploy" ]]; then echo "CCONTRACT$i"; fi
exit 0
EOF
    chmod +x "$STUB_DIR/stellar"

    run bash "$SCRIPT" --source myalias
    [ "$status" -eq 0 ]
  done
}
