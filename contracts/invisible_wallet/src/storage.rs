use soroban_sdk::{contracttype, Env, BytesN};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Signer(BytesN<65>), 
    Guardian,           
}

pub fn add_signer(env: &Env, key: &BytesN<65>) {
    env.storage().persistent().set(&DataKey::Signer(key.clone()), &());
}

pub fn remove_signer(env: &Env, key: &BytesN<65>) {
    env.storage().persistent().remove(&DataKey::Signer(key.clone()));
}

pub fn has_signer(env: &Env, key: &BytesN<65>) -> bool {
    env.storage().persistent().has(&DataKey::Signer(key.clone()))
}

pub fn set_guardian(env: &Env, guardian_key: &BytesN<65>) {
    env.storage().instance().set(&DataKey::Guardian, guardian_key);
}

pub fn get_guardian(env: &Env) -> Option<BytesN<65>> {
    env.storage().instance().get(&DataKey::Guardian)
}
