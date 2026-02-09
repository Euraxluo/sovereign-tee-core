use anyhow::{Result, anyhow};
use k256::FieldBytes;
use k256::Scalar;
use k256::elliptic_curve::PrimeField; // Trait required for from_repr

// Helper to convert the opaque BCS bytes (33 bytes) into a usable Scalar
// This mimics the logic we used in `mock_sign_and_verify`
pub fn bytes_to_scalar(bytes: &[u8]) -> Result<Scalar> {
    let mut key_bytes = bytes.to_vec();

    // 1. Truncate if necessary (BCS might add metadata or it's a 33-byte serialization)
    if key_bytes.len() > 32 {
        key_bytes.resize(32, 0);
    }

    // 2. Reverse endianness (dwallet-mpc uses LE, k256 uses BE)
    key_bytes.reverse();

    // Use Option wrapper
    let scalar_opt = Scalar::from_repr(*FieldBytes::from_slice(&key_bytes));
    if bool::from(scalar_opt.is_some()) {
        Ok(scalar_opt.unwrap())
    } else {
        Err(anyhow!("Invalid scalar encoding"))
    }
}
