use soroban_sdk::{contracttype, Bytes, Env, BytesN};


/// Stores details of a pending guardian recovery request.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingRecovery {
    pub new_public_key: BytesN<65>,
    pub recovery_unlock_time: u64,
}
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Signer(BytesN<65>),
    Guardian,
    /// SHA-256 preimage of the expected rpIdHash (e.g. "localhost" or "veil.app").
    /// Stored at init time; compared against auth_data[0..32] in __check_auth.
    RpId,
    /// The expected WebAuthn origin (e.g. "https://veil.app").
    /// Stored at init time; extracted from clientDataJSON and compared in __check_auth.
    Origin,
}

// â”€â”€ Signer â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn add_signer(env: &Env, key: &BytesN<65>) {
    env.storage().persistent().set(&DataKey::Signer(key.clone()), &());
}

pub fn remove_signer(env: &Env, key: &BytesN<65>) {
    env.storage().persistent().remove(&DataKey::Signer(key.clone()));
}

pub fn has_signer(env: &Env, key: &BytesN<65>) -> bool {
    env.storage().persistent().has(&DataKey::Signer(key.clone()))
}

// â”€â”€ Guardian â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub fn set_guardian(env: &Env, guardian_key: &BytesN<65>) {
    env.storage().instance().set(&DataKey::Guardian, guardian_key);
}

pub fn get_guardian(env: &Env) -> Option<BytesN<65>> {
    env.storage().instance().get(&DataKey::Guardian)
}

// â”€â”€ RP ID â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Persist the relying party ID (e.g. "localhost" for dev, "veil.app" for prod).
pub fn set_rp_id(env: &Env, rp_id: &Bytes) {
    env.storage().instance().set(&DataKey::RpId, rp_id);
}

/// Retrieve the stored relying party ID.
pub fn get_rp_id(env: &Env) -> Option<Bytes> {
    env.storage().instance().get(&DataKey::RpId)
}

// â”€â”€ Origin â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Persist the expected WebAuthn origin (e.g. "https://veil.app").
pub fn set_origin(env: &Env, origin: &Bytes) {
    env.storage().instance().set(&DataKey::Origin, origin);
}

/// Retrieve the stored origin.
pub fn get_origin(env: &Env) -> Option<Bytes> {
    env.storage().instance().get(&DataKey::Origin)
}