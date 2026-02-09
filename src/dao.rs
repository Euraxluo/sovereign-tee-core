use k256::ecdsa::signature::{Signer, Verifier};
use k256::ecdsa::{Signature, SigningKey, VerifyingKey};
use k256::elliptic_curve::group::GroupEncoding;
use serde::{Deserialize, Serialize}; // For to_encoded_point
// Use rand_core explicitly to match k256 dependency requirement
use anyhow::{Result, anyhow};
use rand_core::OsRng;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct Member {
    pub name: String,
    pub pubkey_hex: String,
    // In a real CLI, we might store private keys in separate files,
    // but for this "Scenario Simulation", we keep them here for ease of use.
    pub privkey_hex: String,
}

impl Member {
    pub fn new(name: &str) -> Self {
        // Use OsRng from rand_core v0.6 which implements CryptoRngCore for k256
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = VerifyingKey::from(&signing_key);

        // k256 SigningKey to_bytes returns FieldBytes
        let priv_bytes = signing_key.to_bytes();
        let pub_bytes = verifying_key.to_encoded_point(true).as_bytes().to_vec();

        Self {
            name: name.to_string(),
            privkey_hex: hex::encode(priv_bytes),
            pubkey_hex: hex::encode(pub_bytes),
        }
    }

    pub fn sign(&self, message: &[u8]) -> Result<String> {
        let priv_bytes = hex::decode(&self.privkey_hex)?;
        let signing_key = SigningKey::from_bytes(priv_bytes.as_slice().into())
            .map_err(|e| anyhow!("Invalid private key: {}", e))?;

        let signature: Signature = signing_key.sign(message);
        Ok(hex::encode(signature.to_bytes()))
    }
}

#[derive(Serialize, Deserialize)]
pub struct DaoGroup {
    pub threshold: usize,
    pub members: Vec<Member>,
}

impl DaoGroup {
    pub fn verify_proposal(
        &self,
        message: &[u8],
        signatures: &HashMap<String, String>,
    ) -> Result<bool> {
        let mut valid_votes = 0;

        for (member_name, sig_hex) in signatures {
            if let Some(member) = self.members.iter().find(|m| &m.name == member_name) {
                let pub_bytes = hex::decode(&member.pubkey_hex)?;
                let verifying_key = VerifyingKey::from_sec1_bytes(&pub_bytes)
                    .map_err(|e| anyhow!("Invalid pubkey for {}: {}", member_name, e))?;

                let sig_bytes = hex::decode(sig_hex)?;
                let signature = Signature::from_slice(&sig_bytes)
                    .map_err(|e| anyhow!("Invalid signature format: {}", e))?;

                if verifying_key.verify(message, &signature).is_ok() {
                    valid_votes += 1;
                } else {
                    println!("WARN: Invalid signature from {}", member_name);
                }
            }
        }

        Ok(valid_votes >= self.threshold)
    }
}
