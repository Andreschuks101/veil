#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracterror,
    Env, Address, Bytes, BytesN, Vec, Symbol, Val,
    auth::Context, FromVal, TryIntoVal, symbol_short};

mod auth;
mod storage;
use storage::{DataKey, PendingRecovery};

/// Recovery timelock duration: 3 days in seconds.
const RECOVERY_DELAY_SECONDS: u64 = 259_200;


#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum WalletError {
    AlreadyInitialized          = 1,
    InvalidSignatureFormat      = 2,
    SignerNotAuthorized         = 3,
    InvalidPublicKey            = 4,
    InvalidSignature            = 5,
    SignatureVerificationFailed = 6,
    InvalidChallenge            = 7,
    /// The rpIdHash in authenticatorData does not match SHA-256(stored rp_id).
    /// This means the assertion was produced for a different domain.
    RpIdMismatch                = 8,
    /// The origin field in clientDataJSON does not match the stored origin.
    /// This means the assertion was produced on a different website.
    OriginMismatch              = 9,
}

#[contract]
pub struct InvisibleWallet;

#[contractimpl]
impl InvisibleWallet {
    /// Initialise the wallet with its first signer and domain-binding parameters.
    ///
    /// `rp_id`   â€” the WebAuthn relying party ID (e.g. `"localhost"` for dev,
    ///             `"veil.app"` for production). Must match the domain that
    ///             serves the frontend. Keep it configurable â€” do not hardcode.
    ///
    /// `origin`  â€” the exact WebAuthn origin (e.g. `"https://veil.app"`).
    ///             Must match the `origin` field the browser embeds in every
    ///             clientDataJSON for this deployment.
    pub fn init(
        env: Env,
        initial_signer: BytesN<65>,
        rp_id: Bytes,
        origin: Bytes,
    ) -> Result<(), WalletError> {
        if storage::has_signer(&env, &initial_signer) {
            return Err(WalletError::AlreadyInitialized);
        }
        storage::add_signer(&env, &initial_signer);
        storage::set_rp_id(&env, &rp_id);
        storage::set_origin(&env, &origin);
        Ok(())
    }

    pub fn add_signer(env: Env, new_signer: BytesN<65>) {
        env.current_contract_address().require_auth();
        storage::add_signer(&env, &new_signer);
    }

    pub fn remove_signer(env: Env, signer: BytesN<65>) {
        env.current_contract_address().require_auth();
        storage::remove_signer(&env, &signer);
    }

    pub fn set_guardian(env: Env, guardian: BytesN<65>) {
        env.current_contract_address().require_auth();
        storage::set_guardian(&env, &guardian);
    }

    /// Called by the Soroban runtime to authorize a transaction.
    ///
    /// The `signature` Val must encode a Vec<Val> with 4 elements:
    ///   [0] BytesN<65>  â€” uncompressed P-256 public key (0x04 || x || y)
    ///   [1] Bytes       â€” WebAuthn authenticatorData
    ///   [2] Bytes       â€” WebAuthn clientDataJSON (must contain base64url(signature_payload) as challenge)
    ///   [3] BytesN<64>  â€” raw P-256 ECDSA signature (r || s)
    ///
    /// Verification order:
    ///   1. Parse and validate signature format
    ///   2. Check signer is registered
    ///   3. Verify ECDSA signature + challenge binding  (`verify_webauthn`)
    ///   4. Verify rpIdHash binding                    (`verify_rp_id`)    â†’ RpIdMismatch
    ///   5. Verify origin binding                      (`verify_origin`)   â†’ OriginMismatch
    ///
    /// Steps 4 and 5 run after step 3 so that a bad domain does not produce
    /// a faster failure path than a bad signature (timing side-channel).
    pub fn __check_auth(
        env: Env,
        signature_payload: BytesN<32>,
        signature: Val,
        _auth_contexts: Vec<Context>,
    ) -> Result<(), WalletError> {
        let parts: Vec<Val> = Vec::from_val(&env, &signature);
        if parts.len() != 4 {
            return Err(WalletError::InvalidSignatureFormat);
        }

        let public_key: BytesN<65> = parts
            .get(0).ok_or(WalletError::InvalidSignatureFormat)?
            .try_into_val(&env).map_err(|_| WalletError::InvalidSignatureFormat)?;

        let auth_data: Bytes = parts
            .get(1).ok_or(WalletError::InvalidSignatureFormat)?
            .try_into_val(&env).map_err(|_| WalletError::InvalidSignatureFormat)?;

        let client_data_json: Bytes = parts
            .get(2).ok_or(WalletError::InvalidSignatureFormat)?
            .try_into_val(&env).map_err(|_| WalletError::InvalidSignatureFormat)?;

        let sig_bytes: BytesN<64> = parts
            .get(3).ok_or(WalletError::InvalidSignatureFormat)?
            .try_into_val(&env).map_err(|_| WalletError::InvalidSignatureFormat)?;

        if !storage::has_signer(&env, &public_key) {
            return Err(WalletError::SignerNotAuthorized);
        }

        // Step 3 â€” ECDSA + challenge verification.
        // Clone auth_data and client_data_json so they remain available for
        // the domain-binding checks below.
        auth::verify_webauthn(
            &env,
            &signature_payload,
            public_key,
            auth_data.clone(),
            client_data_json.clone(),
            sig_bytes,
        )?;

        // Step 4 â€” RP ID binding.
        // Ensures auth_data[0..32] == SHA-256(stored rp_id).
        let rp_id = storage::get_rp_id(&env).ok_or(WalletError::RpIdMismatch)?;
        auth::verify_rp_id(&rp_id, &auth_data)?;

        // Step 5 â€” Origin binding.
        // Ensures the "origin" field in clientDataJSON matches the stored origin.
        let origin = storage::get_origin(&env).ok_or(WalletError::OriginMismatch)?;
        auth::verify_origin(&client_data_json, &origin)?;

        Ok(())
    }

    pub fn has_signer(env: Env, key: BytesN<65>) -> bool {
        storage::has_signer(&env, &key)
    }

    pub fn execute(env: Env, target: Address, func: Symbol, args: Vec<Val>) {
        env.current_contract_address().require_auth();
        env.invoke_contract::<Val>(&target, &func, args);
    }

    /// Set or update the guardian address for this wallet.
    ///
    /// Only callable by the current wallet signer (authenticated via __check_auth).
    /// The guardian is authorized to initiate key recovery if the signer key is lost.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment handle.
    /// * `guardian` - The `Address` of the new guardian.
    pub fn set_guardian(env: Env, guardian: Address) {
        // Require that the contract itself (i.e. the wallet signer) authorizes this call.
        env.current_contract_address().require_auth();

        env.storage().persistent().set(&DataKey::Guardian, &guardian);

        env.events().publish(
            (symbol_short!("guardian"), symbol_short!("set")),
            guardian,
        );
    }

    /// Initiate a guardian recovery to replace the wallet signer key.
    ///
    /// Only callable by the designated guardian. Records the new public key
    /// and starts a timelock countdown. After the timelock expires,
    /// `complete_recovery` can be called to finalize the key replacement.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment handle.
    /// * `new_public_key` - The 65-byte uncompressed public key of the new signer.
    ///
    /// # Errors
    /// * `WalletError::NoGuardianSet` - if no guardian has been configured.
    /// * `WalletError::RecoveryAlreadyPending` - if a recovery is already in progress.
    pub fn initiate_recovery(env: Env, new_public_key: BytesN<65>) -> Result<(), WalletError> {
        // Verify a guardian is set
        let guardian: Address = env.storage()
            .persistent()
            .get(&DataKey::Guardian)
            .ok_or(WalletError::NoGuardianSet)?;

        // Require guardian authorization
        guardian.require_auth();

        // Prevent overwriting an existing pending recovery
        if env.storage().persistent().has(&DataKey::RecoveryPending) {
            return Err(WalletError::RecoveryAlreadyPending);
        }

        // Calculate unlock time: current ledger timestamp + 3 day delay
        let recovery_unlock_time = env.ledger().timestamp() + RECOVERY_DELAY_SECONDS;

        let pending = PendingRecovery {
            new_public_key: new_public_key.clone(),
            recovery_unlock_time,
        };

        env.storage().persistent().set(&DataKey::RecoveryPending, &pending);

        env.events().publish(
            (symbol_short!("recovery"), symbol_short!("init")),
            (new_public_key, recovery_unlock_time),
        );

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{Env, Bytes, BytesN, symbol_short};
    use sha2::{Sha256, Digest};
    use p256::ecdsa::{SigningKey, Signature as P256Sig, signature::hazmat::PrehashSigner};

    fn test_keypair() -> (SigningKey, [u8; 65]) {
        let signing_key = SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let encoded = signing_key.verifying_key().to_encoded_point(false);
        let pub_bytes: [u8; 65] = encoded.as_bytes().try_into().unwrap();
        (signing_key, pub_bytes)
    }

    /// Build a minimal valid WebAuthn test fixture for a given payload and signing key.
    /// `rp_id_raw` is used to compute the correct rpIdHash for auth_data[0..32].
    /// Returns (auth_data_bytes, challenge_b64, sig_bytes).
    fn make_webauthn_fixture(
        signing_key: &SigningKey,
        payload: &[u8; 32],
        rp_id_raw: &[u8],
    ) -> ([u8; 37], [u8; 43], [u8; 64]) {
        // auth_data[0..32] = SHA-256(rp_id), flags(1) + signCount(4)
        let rp_id_hash: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(rp_id_raw);
            h.finalize().into()
        };
        let mut auth_data = [0u8; 37];
        auth_data[..32].copy_from_slice(&rp_id_hash);

        // clientDataJSON challenge must be base64url(payload)
        // For payload = [7u8; 32]: base64url = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc"
        let challenge_b64 = *b"BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";

        // Build the full clientDataJSON to hash
        let client_data_json_bytes = build_client_data_json_raw(&challenge_b64);

        let client_data_hash: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(&client_data_json_bytes);
            h.finalize().into()
        };

        // message_hash = SHA256(authData || SHA256(clientDataJSON))
        let message_hash: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(auth_data);
            h.update(client_data_hash);
            h.finalize().into()
        };

        let sig: P256Sig = signing_key.sign_prehash(&message_hash).unwrap();
        let sig_bytes: [u8; 64] = sig.to_bytes().into();

        (auth_data, challenge_b64, sig_bytes)
    }

    /// Build the raw clientDataJSON bytes (for hashing in test fixtures).
    fn build_client_data_json_raw(challenge_b64: &[u8; 43]) -> Vec<u8> {
        let prefix = b"{\"type\":\"webauthn.get\",\"challenge\":\"";
        let suffix = b"\",\"origin\":\"https://test.example\",\"crossOrigin\":false}";
        let mut out = Vec::new();
        out.extend_from_slice(prefix);
        out.extend_from_slice(challenge_b64);
        out.extend_from_slice(suffix);
        out
    }

    /// Build the Soroban Bytes version of clientDataJSON.
    fn build_client_data_json(env: &Env, challenge_b64: &[u8; 43]) -> Bytes {
        let raw = build_client_data_json_raw(challenge_b64);
        let mut cdj = Bytes::new(env);
        for &b in &raw { cdj.push_back(b); }
        cdj
    }

    /// Helper: bytes from a string slice
    fn bytes_from_str(env: &Env, s: &str) -> Bytes {
        let mut b = Bytes::new(env);
        for &byte in s.as_bytes() { b.push_back(byte); }
        b
    }

    // â”€â”€ Existing tests (updated to pass rp_id + origin to init) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    #[test]
    fn test_init_registers_signer() {
        let env = Env::default();
        let contract_id = env.register_contract(None, InvisibleWallet);
        let client = InvisibleWalletClient::new(&env, &contract_id);
        let (_, pub_bytes) = test_keypair();
        let rp_id  = bytes_from_str(&env, "localhost");
        let origin = bytes_from_str(&env, "https://localhost:5173");
        client.init(&BytesN::from_array(&env, &pub_bytes), &rp_id, &origin);
    }

    #[test]
    fn test_init_twice_fails() {
        let env = Env::default();
        let contract_id = env.register_contract(None, InvisibleWallet);
        let client = InvisibleWalletClient::new(&env, &contract_id);
        let (_, pub_bytes) = test_keypair();
        let pub_key = BytesN::from_array(&env, &pub_bytes);
        let rp_id  = bytes_from_str(&env, "localhost");
        let origin = bytes_from_str(&env, "https://localhost:5173");
        client.init(&pub_key, &rp_id, &origin);
        assert_eq!(
            client.try_init(&pub_key, &rp_id, &origin),
            Err(Ok(WalletError::AlreadyInitialized))
        );
    }

    #[test]
    fn test_verify_webauthn_valid() {
        let env = Env::default();
        let (signing_key, pub_bytes) = test_keypair();
        let payload = [7u8; 32];

        let (auth_data_raw, challenge_b64, sig_bytes) =
            make_webauthn_fixture(&signing_key, &payload, b"localhost");

        let result = auth::verify_webauthn(
            &env,
            &BytesN::from_array(&env, &payload),
            BytesN::from_array(&env, &pub_bytes),
            Bytes::from_array(&env, &auth_data_raw),
            build_client_data_json(&env, &challenge_b64),
            BytesN::from_array(&env, &sig_bytes),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_webauthn_wrong_key_fails() {
        let env = Env::default();
        let (signing_key, _) = test_keypair();
        let (_, pub_bytes_wrong) = {
            let k = SigningKey::from_bytes(&[99u8; 32].into()).unwrap();
            let enc = k.verifying_key().to_encoded_point(false);
            let bytes: [u8; 65] = enc.as_bytes().try_into().unwrap();
            (k, bytes)
        };
        let payload = [7u8; 32];

        let (auth_data_raw, challenge_b64, sig_bytes) =
            make_webauthn_fixture(&signing_key, &payload, b"localhost");

        let result = auth::verify_webauthn(
            &env,
            &BytesN::from_array(&env, &payload),
            BytesN::from_array(&env, &pub_bytes_wrong),
            Bytes::from_array(&env, &auth_data_raw),
            build_client_data_json(&env, &challenge_b64),
            BytesN::from_array(&env, &sig_bytes),
        );
        assert_eq!(result, Err(WalletError::SignatureVerificationFailed));
    }

    #[test]
    fn test_verify_webauthn_wrong_challenge_fails() {
        let env = Env::default();
        let (signing_key, pub_bytes) = test_keypair();
        let payload = [7u8; 32];

        let (auth_data_raw, challenge_b64, sig_bytes) =
            make_webauthn_fixture(&signing_key, &payload, b"localhost");

        // Pass a different payload â€” challenge won't match
        let wrong_payload = [8u8; 32];

        let result = auth::verify_webauthn(
            &env,
            &BytesN::from_array(&env, &wrong_payload),
            BytesN::from_array(&env, &pub_bytes),
            Bytes::from_array(&env, &auth_data_raw),
            build_client_data_json(&env, &challenge_b64),
            BytesN::from_array(&env, &sig_bytes),
        );
        assert_eq!(result, Err(WalletError::InvalidChallenge));
    }

    #[test]
    fn test_verify_webauthn_tampered_authdata_fails() {
        let env = Env::default();
        let (signing_key, pub_bytes) = test_keypair();
        let payload = [7u8; 32];

        let (_, challenge_b64, sig_bytes) =
            make_webauthn_fixture(&signing_key, &payload, b"localhost");

        // Use different authData than what was signed
        let tampered_auth_data = [0xffu8; 37];

        let result = auth::verify_webauthn(
            &env,
            &BytesN::from_array(&env, &payload),
            BytesN::from_array(&env, &pub_bytes),
            Bytes::from_array(&env, &tampered_auth_data),
            build_client_data_json(&env, &challenge_b64),
            BytesN::from_array(&env, &sig_bytes),
        );
        assert_eq!(result, Err(WalletError::SignatureVerificationFailed));
    }

    // â”€â”€ New tests: domain binding â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// RpIdMismatch: auth_data[0..32] is SHA-256("localhost") but we store "veil.app".
    #[test]
    fn test_rp_id_mismatch() {
        let env = Env::default();

        // auth_data is built with SHA-256("localhost") as the rpIdHash
        let rp_id_hash: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(b"localhost");
            h.finalize().into()
        };
        let mut auth_data = [0u8; 37];
        auth_data[..32].copy_from_slice(&rp_id_hash);

        // But stored rp_id is "veil.app" â€” different domain
        let stored_rp_id = bytes_from_str(&env, "veil.app");

        let auth_data_bytes = {
            let mut b = Bytes::new(&env);
            for &byte in &auth_data { b.push_back(byte); }
            b
        };

        let result = auth::verify_rp_id(&stored_rp_id, &auth_data_bytes);
        assert_eq!(result, Err(WalletError::RpIdMismatch));
    }

    /// OriginMismatch: clientDataJSON has `"origin":"https://test.example"` but
    /// stored origin is `"https://veil.app"`.
    #[test]
    fn test_origin_mismatch() {
        let env = Env::default();

        // clientDataJSON embeds origin = "https://test.example"
        let challenge_b64 = *b"BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";
        let client_data_json = build_client_data_json(&env, &challenge_b64);

        // But stored origin is "https://veil.app"
        let stored_origin = bytes_from_str(&env, "https://veil.app");

        let result = auth::verify_origin(&client_data_json, &stored_origin);
        assert_eq!(result, Err(WalletError::OriginMismatch));
    }

    /// Sanity check: verify_rp_id passes when rp_id matches auth_data[0..32].
    #[test]
    fn test_rp_id_match() {
        let env = Env::default();

        let rp_id_hash: [u8; 32] = {
            let mut h = Sha256::new();
            h.update(b"localhost");
            h.finalize().into()
        };
        let mut auth_data = [0u8; 37];
        auth_data[..32].copy_from_slice(&rp_id_hash);

        let stored_rp_id = bytes_from_str(&env, "localhost");
        let auth_data_bytes = {
            let mut b = Bytes::new(&env);
            for &byte in &auth_data { b.push_back(byte); }
            b
        };

        let result = auth::verify_rp_id(&stored_rp_id, &auth_data_bytes);
        assert!(result.is_ok());
    }

    /// Sanity check: verify_origin passes when origin matches the stored value.
    #[test]
    fn test_origin_match() {
        let env = Env::default();

        let challenge_b64 = *b"BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";
        let client_data_json = build_client_data_json(&env, &challenge_b64);

        // clientDataJSON has origin "https://test.example" â€” store the same
        let stored_origin = bytes_from_str(&env, "https://test.example");

        let result = auth::verify_origin(&client_data_json, &stored_origin);
        assert!(result.is_ok());
    }
}