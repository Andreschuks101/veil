use soroban_sdk::BytesN;
use crate::FactoryError;

/// Returns Ok(()) if public_key is a valid uncompressed P-256 point.
/// Checks: first byte == 0x04, then VerifyingKey::from_sec1_bytes.
pub fn validate_public_key(public_key: &BytesN<65>) -> Result<(), FactoryError> {
    let bytes = public_key.to_array();

    if bytes[0] != 0x04 {
        return Err(FactoryError::InvalidPublicKey);
    }

    p256::ecdsa::VerifyingKey::from_sec1_bytes(&bytes)
        .map_err(|_| FactoryError::InvalidPublicKey)?;

    Ok(())
}
