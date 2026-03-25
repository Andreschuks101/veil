#![no_std]
use soroban_sdk::{contract, contractimpl, contracterror, symbol_short, Env, Address, BytesN, Vec, IntoVal};

mod storage;
mod validation;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum FactoryError {
    AlreadyInitialized = 1,
    NotInitialized     = 2,
    AlreadyDeployed    = 3,
    InvalidPublicKey   = 4,
}

#[contract]
pub struct Factory;

#[contractimpl]
impl Factory {
    /// One-time initialization. Stores the wallet Wasm hash.
    pub fn init(env: Env, wasm_hash: BytesN<32>) -> Result<(), FactoryError> {
        if storage::has_wasm_hash(&env) {
            return Err(FactoryError::AlreadyInitialized);
        }
        storage::set_wasm_hash(&env, &wasm_hash);
        Ok(())
    }

    /// Deploy a new invisible_wallet for the given P-256 public key.
    /// Returns the Address of the newly deployed wallet.
    pub fn deploy(env: Env, public_key: BytesN<65>) -> Result<Address, FactoryError> {
        // Step 1: must be initialized
        let wasm_hash = storage::get_wasm_hash(&env)
            .ok_or(FactoryError::NotInitialized)?;

        // Step 2: validate public key
        validation::validate_public_key(&public_key)?;

        // Step 3: compute salt = SHA-256(public_key_bytes)
        let key_bytes = public_key.to_array();
        let salt_bytes = sha2_hash(&key_bytes);
        let salt = BytesN::from_array(&env, &salt_bytes);

        // Step 4: check for duplicate
        if storage::is_deployed(&env, &salt) {
            return Err(FactoryError::AlreadyDeployed);
        }

        // Step 5: deploy the contract and call init atomically
        let wallet_address = env
            .deployer()
            .with_address(env.current_contract_address(), salt.clone())
            .deploy(wasm_hash);

        let init_args: Vec<soroban_sdk::Val> = (public_key.clone(),).into_val(&env);
        env.invoke_contract::<soroban_sdk::Val>(&wallet_address, &symbol_short!("init"), init_args);

        // Step 6: mark as deployed
        storage::mark_deployed(&env, &salt);

        // Step 7: return address
        Ok(wallet_address)
    }
}

/// Compute SHA-256 of a 65-byte input, returning a 32-byte array.
fn sha2_hash(input: &[u8; 65]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().into()
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{Env, BytesN};

    fn make_env() -> Env {
        Env::default()
    }

    fn dummy_wasm_hash(env: &Env) -> BytesN<32> {
        BytesN::from_array(env, &[1u8; 32])
    }

    fn valid_pub_key(env: &Env) -> BytesN<65> {
        use p256::ecdsa::SigningKey;
        let signing_key = SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let encoded = signing_key.verifying_key().to_encoded_point(false);
        let bytes: [u8; 65] = encoded.as_bytes().try_into().unwrap();
        BytesN::from_array(env, &bytes)
    }

    #[test]
    fn test_init_stores_wasm_hash() {
        let env = make_env();
        let contract_id = env.register_contract(None, Factory);
        let client = FactoryClient::new(&env, &contract_id);
        let hash = dummy_wasm_hash(&env);
        client.init(&hash);
        env.as_contract(&contract_id, || {
            assert_eq!(storage::get_wasm_hash(&env).unwrap(), hash);
        });
    }

    #[test]
    fn test_double_init_fails() {
        let env = make_env();
        let contract_id = env.register_contract(None, Factory);
        let client = FactoryClient::new(&env, &contract_id);
        let hash = dummy_wasm_hash(&env);
        client.init(&hash);
        assert_eq!(
            client.try_init(&hash),
            Err(Ok(FactoryError::AlreadyInitialized))
        );
    }

    #[test]
    fn test_deploy_before_init_fails() {
        let env = make_env();
        let contract_id = env.register_contract(None, Factory);
        let client = FactoryClient::new(&env, &contract_id);
        let pub_key = valid_pub_key(&env);
        assert_eq!(
            client.try_deploy(&pub_key),
            Err(Ok(FactoryError::NotInitialized))
        );
    }

    #[test]
    fn test_invalid_public_key_bad_prefix() {
        let env = make_env();
        let contract_id = env.register_contract(None, Factory);
        let client = FactoryClient::new(&env, &contract_id);
        client.init(&dummy_wasm_hash(&env));
        let mut bad_key = [0u8; 65];
        bad_key[0] = 0x03;
        let pub_key = BytesN::from_array(&env, &bad_key);
        assert_eq!(
            client.try_deploy(&pub_key),
            Err(Ok(FactoryError::InvalidPublicKey))
        );
    }

    #[test]
    fn test_duplicate_deploy_prevented() {
        // Verifies the AlreadyDeployed guard fires before reaching the deployer.
        // We call deploy twice; the second call must fail with AlreadyDeployed
        // because the salt is already marked in storage after the first attempt
        // reaches the duplicate check (even if the actual wasm deploy would fail
        // without a real wasm hash — the guard fires first on the second call).
        let env = make_env();
        let contract_id = env.register_contract(None, Factory);
        let client = FactoryClient::new(&env, &contract_id);
        client.init(&dummy_wasm_hash(&env));

        let pub_key = valid_pub_key(&env);

        // Manually mark the salt as deployed to simulate a prior successful deploy
        let key_bytes = pub_key.to_array();
        let salt_bytes = {
            use sha2::{Sha256, Digest};
            let mut h = Sha256::new();
            h.update(key_bytes);
            let r: [u8; 32] = h.finalize().into();
            r
        };
        let salt = BytesN::from_array(&env, &salt_bytes);
        env.as_contract(&contract_id, || {
            storage::mark_deployed(&env, &salt);
        });

        // Now deploy should return AlreadyDeployed
        assert_eq!(
            client.try_deploy(&pub_key),
            Err(Ok(FactoryError::AlreadyDeployed))
        );
    }
}
