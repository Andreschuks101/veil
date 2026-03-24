# Requirements Document

## Introduction

The factory contract is a permissionless Soroban smart contract that automates deployment of `invisible_wallet` contract instances on Stellar. Each user's wallet is a separate deployed contract identified by a deterministic address derived solely from their P-256 public key. The factory eliminates the need for manual wallet deployment, enabling self-service onboarding for Veil passkey wallet users.

## Glossary

- **Factory**: The Soroban smart contract at `contracts/factory/` that deploys wallet instances.
- **Wallet**: An instance of the `invisible_wallet` contract — a custom Soroban account contract that verifies P-256 ECDSA signatures.
- **Public_Key**: An uncompressed P-256 ECDSA public key encoded as 65 bytes (`0x04 || x || y`).
- **Salt**: A 32-byte value derived deterministically from a Public_Key, used to produce a predictable contract address via `env.deployer()`.
- **Deployer**: The `soroban_sdk::deployer` API used to deploy and initialize Wasm contracts on-chain.
- **Wasm_Hash**: The SHA-256 hash of the compiled `invisible_wallet` contract Wasm blob, registered on-chain before deployment.
- **Signer**: A Public_Key registered inside a Wallet instance that is authorized to sign transactions for that wallet.
- **FactoryError**: The `contracterror` enum defined in the factory contract for structured on-chain error codes.

## Requirements

### Requirement 1: Deploy Wallet Instance

**User Story:** As a Veil user, I want to call a single on-chain entry point with my public key, so that my personal wallet contract is deployed and initialized without any manual intervention.

#### Acceptance Criteria

1. THE Factory SHALL expose a `deploy(public_key: BytesN<65>)` entry point callable by any account.
2. WHEN `deploy` is called with a valid 65-byte uncompressed P-256 Public_Key, THE Factory SHALL deploy a new Wallet contract instance using `env.deployer()`.
3. WHEN `deploy` is called with a valid Public_Key, THE Factory SHALL initialize the deployed Wallet by invoking its `init` function with the provided Public_Key as the initial Signer.
4. WHEN `deploy` succeeds, THE Factory SHALL return the `Address` of the newly deployed Wallet contract.

### Requirement 2: Deterministic Address Derivation

**User Story:** As a Veil frontend developer, I want the wallet address to be predictable from the public key alone, so that I can compute the wallet address off-chain before deployment and display it to the user.

#### Acceptance Criteria

1. THE Factory SHALL derive the Salt for each deployment as the SHA-256 hash of the raw 65-byte Public_Key bytes.
2. THE Factory SHALL pass the derived Salt to `env.deployer().with_address(deployer_address, salt)` so the resulting contract address is fully determined by the Public_Key.
3. FOR ALL valid Public_Keys, computing the Salt off-chain as `SHA-256(public_key_bytes)` and applying the same Soroban address derivation formula SHALL produce the same address as an on-chain `deploy` call with that Public_Key.

### Requirement 3: Duplicate Deployment Prevention

**User Story:** As a Veil protocol designer, I want deploying the same public key twice to fail with a clear error, so that wallet uniqueness is enforced on-chain and no key can be associated with more than one wallet.

#### Acceptance Criteria

1. WHEN `deploy` is called with a Public_Key for which a Wallet already exists, THE Factory SHALL return `FactoryError::AlreadyDeployed`.
2. WHEN `deploy` is called with a Public_Key for which a Wallet already exists, THE Factory SHALL NOT deploy a new contract instance.
3. WHEN `deploy` is called with a Public_Key for which a Wallet already exists, THE Factory SHALL NOT invoke `init` on any contract.

### Requirement 4: Public Key Validation

**User Story:** As a Veil protocol designer, I want the factory to reject malformed public keys before attempting deployment, so that wasted deployment fees and inconsistent on-chain state are avoided.

#### Acceptance Criteria

1. WHEN `deploy` is called with a Public_Key whose first byte is not `0x04`, THE Factory SHALL return `FactoryError::InvalidPublicKey`.
2. WHEN `deploy` is called with a Public_Key that is not a valid point on the P-256 curve, THE Factory SHALL return `FactoryError::InvalidPublicKey`.
3. WHEN `deploy` is called with a valid Public_Key, THE Factory SHALL proceed with deployment without returning `FactoryError::InvalidPublicKey`.

### Requirement 5: Wasm Hash Configuration

**User Story:** As a Veil contract operator, I want the factory to reference the correct wallet Wasm hash at construction time, so that all deployed wallets run the same audited contract code.

#### Acceptance Criteria

1. THE Factory SHALL expose an `init(wasm_hash: BytesN<32>)` entry point that stores the Wasm_Hash used for all subsequent deployments.
2. WHEN `init` is called more than once, THE Factory SHALL return `FactoryError::AlreadyInitialized`.
3. WHILE the Factory has not been initialized, THE Factory SHALL return `FactoryError::NotInitialized` when `deploy` is called.
4. THE Factory SHALL use the stored Wasm_Hash in every `env.deployer()` call made during `deploy`.

### Requirement 6: Error Reporting

**User Story:** As a Veil SDK developer, I want all factory failures to surface as typed on-chain error codes, so that client code can distinguish between error conditions programmatically.

#### Acceptance Criteria

1. THE Factory SHALL define a `FactoryError` enum using `#[contracterror]` with at minimum the variants: `AlreadyInitialized`, `NotInitialized`, `AlreadyDeployed`, and `InvalidPublicKey`.
2. WHEN any entry point encounters an error condition, THE Factory SHALL return the corresponding `FactoryError` variant rather than panicking.
3. THE Factory SHALL assign each `FactoryError` variant a unique non-zero `u32` discriminant.
