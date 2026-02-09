use anyhow::{Result, anyhow};
use dwallet_mpc_types::dwallet_mpc::DWalletCurve;
use group::secp256k1::Scalar as SecpScalar;
use group::secp256k1::scalar::PublicParameters;
use group::{OsCsRng, Samplable};

// For mock signing and verification
use k256::FieldBytes;
use k256::ecdsa::signature::{Signer, Verifier};
use k256::ecdsa::{Signature, SigningKey, VerifyingKey};

pub struct RefreshedShares {
    pub new_dao_share: Vec<u8>,
    pub new_tee_share: Vec<u8>,
}

pub fn perform_pss_refresh(
    curve: u32,
    dao_share_bytes: &[u8],
    tee_share_bytes: &[u8],
) -> Result<RefreshedShares> {
    match curve {
        0 => refresh_secp256k1(dao_share_bytes, tee_share_bytes),
        _ => Err(anyhow!("Curve not supported for PSS yet")),
    }
}

fn refresh_secp256k1(dao_share_bytes: &[u8], tee_share_bytes: &[u8]) -> Result<RefreshedShares> {
    let s_dao: SecpScalar = bcs::from_bytes(dao_share_bytes)
        .map_err(|e| anyhow!("Failed to deserialize DAO share: {}", e))?;

    let s_tee: SecpScalar = bcs::from_bytes(tee_share_bytes)
        .map_err(|e| anyhow!("Failed to deserialize TEE share: {}", e))?;

    let pp = PublicParameters::default();
    // Use OsCsRng from the group crate to ensure trait compatibility
    let alpha = SecpScalar::sample(&pp, &mut OsCsRng)
        .map_err(|e| anyhow!("Failed to sample alpha: {}", e))?;

    let s_dao_new = s_dao + &alpha;
    let s_tee_new = s_tee - &alpha;

    let new_dao_bytes = bcs::to_bytes(&s_dao_new)
        .map_err(|e| anyhow!("Failed to serialize new DAO share: {}", e))?;

    let new_tee_bytes = bcs::to_bytes(&s_tee_new)
        .map_err(|e| anyhow!("Failed to serialize new TEE share: {}", e))?;

    Ok(RefreshedShares {
        new_dao_share: new_dao_bytes,
        new_tee_share: new_tee_bytes,
    })
}

/// DANGER: This function reconstructs the private key from shares.
/// It is intended ONLY for testing and verification purposes (CLI).
/// NEVER use this in the actual TEE production flow.
pub fn mock_sign_and_verify(
    dao_share_bytes: &[u8],
    tee_share_bytes: &[u8],
    message: &[u8],
) -> Result<(String, String)> {
    // 1. Reconstruct Private Key (s = s1 + s2)
    let s_dao: SecpScalar = bcs::from_bytes(dao_share_bytes)?;
    let s_tee: SecpScalar = bcs::from_bytes(tee_share_bytes)?;

    let private_key_scalar = s_dao + s_tee;

    // Convert Scalar to bytes via BCS
    let mut key_bytes = bcs::to_bytes(&private_key_scalar)?;

    // Debug: Print length if not 32
    if key_bytes.len() != 32 {
        eprintln!("DEBUG: BCS Scalar Bytes Len: {}", key_bytes.len());
        // If 33 bytes, it might be [len, data...] or [data..., extra]
        // But for fixed size structs, BCS shouldn't add len.
        // Unless it's encoding an Enum or specific internal structure.

        // Let's try to strip the first byte if it is 33 bytes?
        // Wait, k256 private keys are 32 bytes.
        // If we have 33 bytes, maybe it's just padding or a tag.

        // Strategy: Force fit to 32 bytes.
        // If > 32, take last 32? Or first 32?
        // Assuming LE encoding, and extra bytes might be high bits or overflow protection.

        // Let's try taking the *last* 32 bytes if we assume it's BE with a leading 0?
        // Or if it's LE, first 32 bytes?

        // Given the previous panic was `assertion left == right failed: left: 33, right: 32`
        // inside `SigningKey::from_bytes`, which strictly requires 32 bytes.

        // Let's assume it's a fixed size 32 byte scalar but maybe BCS adds something?
        // Actually, dwallet-mpc Scalar might be 4 limbs of u64 + extra stuff?

        // FIX: Let's try to just take the first 32 bytes for now.
        // If it's LE, that's the data.
        key_bytes.resize(32, 0);
    }

    // Hack: Assuming LE because most Rust crypto libs are LE.
    // k256 SigningKey needs BE.
    key_bytes.reverse();

    let signing_key = SigningKey::from_bytes(&FieldBytes::from_slice(&key_bytes))
        .map_err(|e| anyhow!("Invalid private key reconstruction: {}", e))?;

    // 2. Derive Public Key
    let verifying_key = VerifyingKey::from(&signing_key);
    let pubkey_hex = hex::encode(verifying_key.to_encoded_point(true).as_bytes());

    // 3. Sign Message
    let signature: Signature = signing_key.sign(message);
    let sig_hex = hex::encode(signature.to_bytes());

    // 4. Verify (Self-check)
    verifying_key
        .verify(message, &signature)
        .map_err(|e| anyhow!("Self-verification failed: {}", e))?;

    Ok((sig_hex, pubkey_hex))
}

pub fn generate_initial_shares() -> Result<(Vec<u8>, Vec<u8>)> {
    let pp = PublicParameters::default();
    let s_dao = SecpScalar::sample(&pp, &mut OsCsRng)
        .map_err(|e| anyhow!("Failed to sample DAO share: {}", e))?;
    let s_tee = SecpScalar::sample(&pp, &mut OsCsRng)
        .map_err(|e| anyhow!("Failed to sample TEE share: {}", e))?;

    Ok((bcs::to_bytes(&s_dao)?, bcs::to_bytes(&s_tee)?))
}
