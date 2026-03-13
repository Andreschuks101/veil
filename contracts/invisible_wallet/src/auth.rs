use soroban_sdk::{Env, Bytes, BytesN};
use p256::ecdsa::{VerifyingKey, Signature, signature::hazmat::PrehashVerifier};
use sha2::{Sha256, Digest};
use crate::WalletError;

const BASE64URL: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Base64url-encode exactly 32 bytes without padding.
/// 32 bytes → always 43 output chars: 10 full groups of 3 + 1 group of 2.
fn base64url_encode_32(input: &[u8; 32]) -> [u8; 43] {
    let mut out = [0u8; 43];
    let mut o = 0usize;
    let mut i = 0usize;
    while i + 3 <= 30 {
        let b0 = input[i] as u32;
        let b1 = input[i + 1] as u32;
        let b2 = input[i + 2] as u32;
        out[o]     = BASE64URL[((b0 >> 2) & 0x3f) as usize];
        out[o + 1] = BASE64URL[(((b0 << 4) | (b1 >> 4)) & 0x3f) as usize];
        out[o + 2] = BASE64URL[(((b1 << 2) | (b2 >> 6)) & 0x3f) as usize];
        out[o + 3] = BASE64URL[(b2 & 0x3f) as usize];
        i += 3;
        o += 4;
    }
    // Final 2 bytes (input[30], input[31]) → 3 output chars, no padding
    let b0 = input[30] as u32;
    let b1 = input[31] as u32;
    out[40] = BASE64URL[((b0 >> 2) & 0x3f) as usize];
    out[41] = BASE64URL[(((b0 << 4) | (b1 >> 4)) & 0x3f) as usize];
    out[42] = BASE64URL[((b1 << 2) & 0x3f) as usize];
    out
}

/// Verify that the base64url(signature_payload) string appears inside clientDataJSON.
/// The WebAuthn spec embeds the challenge as a base64url string in the JSON, so this
/// confirms the assertion was specifically for this Soroban auth payload.
fn challenge_is_present(client_data_json: &Bytes, signature_payload: &[u8; 32]) -> bool {
    let needle = base64url_encode_32(signature_payload);
    let n_len = needle.len(); // 43
    let h_len = client_data_json.len() as usize;
    if h_len < n_len {
        return false;
    }
    'outer: for start in 0..=(h_len - n_len) {
        for j in 0..n_len {
            if client_data_json.get_unchecked((start + j) as u32) != needle[j] {
                continue 'outer;
            }
        }
        return true;
    }
    false
}

/// Verify a full WebAuthn ES256 assertion against a Soroban signature_payload.
///
/// The authenticator signs SHA256(authData || SHA256(clientDataJSON)).
/// The clientDataJSON must contain base64url(signature_payload) as its challenge field,
/// binding this assertion to the exact Soroban authorization entry being authorized.
pub fn verify_webauthn(
    _env: &Env,
    signature_payload: &BytesN<32>,
    public_key: BytesN<65>,
    auth_data: Bytes,
    client_data_json: Bytes,
    signature: BytesN<64>,
) -> Result<(), WalletError> {
    // 1. Verify the challenge in clientDataJSON is base64url(signature_payload)
    if !challenge_is_present(&client_data_json, &signature_payload.to_array()) {
        return Err(WalletError::InvalidChallenge);
    }

    // 2. SHA256(clientDataJSON)
    let client_data_hash: [u8; 32] = {
        let mut h = Sha256::new();
        for i in 0..client_data_json.len() {
            h.update([client_data_json.get_unchecked(i)]);
        }
        h.finalize().into()
    };

    // 3. SHA256(authData || SHA256(clientDataJSON))
    //    This is exactly what the WebAuthn authenticator signed under ES256.
    let message_hash: [u8; 32] = {
        let mut h = Sha256::new();
        for i in 0..auth_data.len() {
            h.update([auth_data.get_unchecked(i)]);
        }
        h.update(client_data_hash);
        h.finalize().into()
    };

    // 4. Verify P-256 ECDSA signature over the message hash
    let pk_bytes: [u8; 65] = public_key.to_array();
    let verifying_key = VerifyingKey::from_sec1_bytes(&pk_bytes)
        .map_err(|_| WalletError::InvalidPublicKey)?;

    let sig_bytes: [u8; 64] = signature.to_array();
    let sig_obj = Signature::from_bytes(&sig_bytes.into())
        .map_err(|_| WalletError::InvalidSignature)?;

    verifying_key.verify_prehash(&message_hash, &sig_obj)
        .map_err(|_| WalletError::SignatureVerificationFailed)
}
