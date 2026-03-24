# Implementation Plan: factory-contract

## Overview

Implement the factory contract as three Rust source files (`lib.rs`, `storage.rs`, `validation.rs`) plus a `Cargo.toml`, all under `contracts/factory/`. Tasks proceed from scaffolding → error types → storage → validation → core logic → tests, ensuring each step compiles and integrates before the next begins.

## Tasks

- [ ] 1. Scaffold the factory crate
  - Create `contracts/factory/Cargo.toml` modelled on `invisible_wallet/Cargo.toml`
  - Add `soroban-sdk`, `p256` (no-std, ecdsa+sha2 features), and `sha2` (no-std) as `[dependencies]`
  - Add `soroban-sdk` (testutils), `p256` (std), `sha2`, and `proptest = "1"` as `[dev-dependencies]`
  - Set `crate-type = ["cdylib"]` and `edition = "2021"`
  - Create empty `contracts/factory/src/lib.rs`, `contracts/factory/src/storage.rs`, and `contracts/factory/src/validation.rs`
  - Verify the crate compiles with `cargo build --target wasm32-unknown-unknown --release -p factory`
  - _Requirements: 1.1, 5.1_

- [ ] 2. Define `FactoryError` and the contract skeleton
  - [ ] 2.1 Add `FactoryError` enum to `lib.rs`
    - Annotate with `#[contracterror]`, `#[derive(Copy, Clone, Debug, Eq, PartialEq)]`, and `#[repr(u32)]`
    - Define variants: `AlreadyInitialized = 1`, `NotInitialized = 2`, `AlreadyDeployed = 3`, `InvalidPublicKey = 4`
    - _Requirements: 6.1, 6.3_
  - [ ] 2.2 Add `Factory` struct and empty `#[contractimpl]` block to `lib.rs`
    - Declare `pub fn init(env: Env, wasm_hash: BytesN<32>) -> Result<(), FactoryError>` returning `unimplemented!()`
    - Declare `pub fn deploy(env: Env, public_key: BytesN<65>) -> Result<Address, FactoryError>` returning `unimplemented!()`
    - _Requirements: 1.1, 5.1_
  - [ ]\* 2.3 Write unit test `test_error_discriminants_unique_nonzero`
    - Cast each `FactoryError` variant to `u32` and assert all values are distinct and non-zero
    - _Requirements: 6.3_

- [ ] 3. Implement the storage module
  - [ ] 3.1 Define `DataKey` enum in `storage.rs`
    - Annotate with `#[contracttype]`
    - Variants: `WasmHash` and `Deployed(BytesN<32>)`
    - _Requirements: 5.4, 3.1_
  - [ ] 3.2 Implement storage helpers in `storage.rs`
    - `set_wasm_hash(env: &Env, hash: &BytesN<32>)` — writes to `instance()` storage
    - `get_wasm_hash(env: &Env) -> Option<BytesN<32>>` — reads from `instance()` storage
    - `has_wasm_hash(env: &Env) -> bool` — checks presence in `instance()` storage
    - `mark_deployed(env: &Env, salt: &BytesN<32>)` — sets `Deployed(salt)` key in `instance()` storage
    - `is_deployed(env: &Env, salt: &BytesN<32>) -> bool` — checks `Deployed(salt)` presence
    - _Requirements: 5.1, 5.4, 3.1, 3.2_
  - [ ]\* 3.3 Write unit test `test_init_stores_wasm_hash`
    - Call `init` on a test environment, then assert `get_wasm_hash` returns the same hash
    - _Requirements: 5.1_

- [ ] 4. Implement the validation module
  - [ ] 4.1 Implement `validate_public_key` in `validation.rs`
    - Accept `public_key: &BytesN<65>`, copy bytes to a `[u8; 65]` array
    - Return `Err(FactoryError::InvalidPublicKey)` if first byte is not `0x04`
    - Call `p256::ecdsa::VerifyingKey::from_sec1_bytes(&bytes)` and map any error to `Err(FactoryError::InvalidPublicKey)`
    - Return `Ok(())` on success
    - _Requirements: 4.1, 4.2, 4.3_
  - [ ]\* 4.2 Write property test for `validate_public_key` — Property 5
    - `// Feature: factory-contract, Property 5: malformed public key returns InvalidPublicKey`
    - Strategy A: generate random `[u8; 65]` with first byte forced to any value except `0x04`; assert `Err(InvalidPublicKey)`
    - Strategy B: generate `[0x04u8]` ++ random `[u8; 64]` that fails `VerifyingKey::from_sec1_bytes`; assert `Err(InvalidPublicKey)`
    - _Requirements: 4.1, 4.2_

- [ ] 5. Implement `init` entry point
  - [ ] 5.1 Fill in `Factory::init` in `lib.rs`
    - Call `storage::has_wasm_hash(&env)`; if true return `Err(FactoryError::AlreadyInitialized)`
    - Call `storage::set_wasm_hash(&env, &wasm_hash)` and return `Ok(())`
    - _Requirements: 5.1, 5.2_
  - [ ]\* 5.2 Write property test for `init` — Property 6
    - `// Feature: factory-contract, Property 6: double init returns AlreadyInitialized`
    - Strategy: generate random `[u8; 32]` wasm hash, call `init` twice on the same env
    - Assert second call returns `Err(FactoryError::AlreadyInitialized)`
    - _Requirements: 5.2_

- [ ] 6. Implement `deploy` entry point
  - [ ] 6.1 Fill in `Factory::deploy` in `lib.rs` following the design control flow
    - Step 1: `storage::get_wasm_hash` → `None` → `Err(NotInitialized)`
    - Step 2: `validation::validate_public_key(&public_key)` → propagate `Err`
    - Step 3: compute `salt` as `SHA-256(public_key.to_array())` using `sha2::Sha256`; convert to `BytesN<32>`
    - Step 4: `storage::is_deployed(&env, &salt)` → true → `Err(AlreadyDeployed)`
    - Step 5: call `env.deployer().with_address(env.current_contract_address(), salt.clone()).deploy_v2(wasm_hash, (public_key,))`
    - Step 6: `storage::mark_deployed(&env, &salt)`
    - Step 7: return `Ok(wallet_address)`
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 3.1, 3.2, 3.3, 5.3, 5.4, 6.2_
  - [ ]\* 6.2 Write property test for `deploy` — Property 7
    - `// Feature: factory-contract, Property 7: deploy before init returns NotInitialized`
    - Strategy: fresh factory env (no `init` call), generate random valid P-256 key
    - Assert `deploy` returns `Err(FactoryError::NotInitialized)`
    - _Requirements: 5.3_
  - [ ]\* 6.3 Write property test for `deploy` — Property 1
    - `// Feature: factory-contract, Property 1: valid key produces a deployed wallet`
    - Strategy: generate random `p256::SigningKey`, extract uncompressed 65-byte public key
    - Assert `factory.deploy(key)` returns `Ok(addr)`
    - _Requirements: 1.2, 1.4_
  - [ ]\* 6.4 Write property test for `deploy` — Property 4
    - `// Feature: factory-contract, Property 4: duplicate deploy returns AlreadyDeployed`
    - Strategy: generate random valid key, call `deploy` twice on the same initialized factory
    - Assert second call returns `Err(FactoryError::AlreadyDeployed)`
    - _Requirements: 3.1, 3.2, 3.3_

- [ ] 7. Checkpoint — verify core logic compiles and passes
  - Ensure `cargo build --target wasm32-unknown-unknown --release -p factory` succeeds with no warnings
  - Ensure all non-optional tests pass with `cargo test -p factory`
  - Ask the user if any questions arise before continuing

- [ ] 8. Implement integration and round-trip tests
  - [ ] 8.1 Write unit test `test_deploy_full_integration`
    - Init factory with a mock wasm hash, deploy a wallet, verify the returned address is non-empty
    - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - [ ]\* 8.2 Write property test for `deploy` — Property 2 (round-trip signer check)
    - `// Feature: factory-contract, Property 2: deployed wallet has key registered as signer`
    - Strategy: generate random valid P-256 key, deploy wallet via factory
    - Assert `InvisibleWalletClient::has_signer(&env, &wallet_addr, &public_key)` returns `true`
    - _Requirements: 1.3_
  - [ ]\* 8.3 Write property test for `deploy` — Property 3 (deterministic address)
    - `// Feature: factory-contract, Property 3: address is deterministic from public key`
    - Strategy: generate random valid key, compute expected salt off-chain as `SHA-256(key_bytes)`
    - Assert address returned by `deploy` equals address derived via Soroban testutils with the same salt
    - _Requirements: 2.1, 2.2, 2.3_
  - [ ]\* 8.4 Write unit test `test_factory_error_all_variants_present`
    - Compile-time exhaustive match over `FactoryError` covering all four variants
    - _Requirements: 6.1_

- [ ] 9. Final checkpoint — all tests pass
  - Ensure `cargo test -p factory` passes (all non-optional tests)
  - Ensure `cargo build --target wasm32-unknown-unknown --release -p factory` produces a `.wasm` artifact
  - Ask the user if any questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP
- Each task references specific requirements for traceability
- Property tests require the `proptest` dev-dependency and run inside `#[cfg(test)]` modules in `lib.rs`
- The `p256` crate is used in both production code (no-std, validation) and tests (std, key generation)
- `deploy_v2` is the Soroban SDK method that atomically deploys and calls `init` on the new contract
