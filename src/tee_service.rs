use anyhow::{Result, anyhow};
use dwallet_mpc_centralized_party::{
    advance_centralized_sign_party, create_dkg_output_by_curve_v2,
    encrypt_secret_key_share_and_prove_v2, generate_cg_keypair_from_seed,
};
use serde::{Deserialize, Serialize};

/// Represents the TEE's local storage of the key material
#[derive(Serialize, Deserialize, Clone)]
pub struct TeeKeyStore {
    pub share_encryption_key: Vec<u8>,
    pub share_decryption_key: Vec<u8>,
    pub dwallet_secret_share: Option<Vec<u8>>,
}

pub struct TeeMpcService {
    curve: u32,
}

impl TeeMpcService {
    pub fn new(curve: u32) -> Self {
        Self { curve }
    }

    pub fn generate_encryption_keypair(&self, seed: [u8; 32]) -> Result<(Vec<u8>, Vec<u8>)> {
        generate_cg_keypair_from_seed(self.curve, seed)
            .map_err(|e| anyhow!("Failed to generate encryption keypair: {}", e))
    }

    pub fn initiate_dkg(&self, protocol_pp: Vec<u8>, session_id: Vec<u8>) -> Result<Vec<u8>> {
        let res = create_dkg_output_by_curve_v2(self.curve, protocol_pp, session_id)
            .map_err(|e| anyhow!("DKG creation failed: {}", e))?;

        Ok(res.public_output)
    }

    pub fn re_encrypt_share(
        &self,
        secret_share: Vec<u8>,
        new_encryption_key_public: Vec<u8>,
        protocol_pp: Vec<u8>,
    ) -> Result<Vec<u8>> {
        encrypt_secret_key_share_and_prove_v2(
            self.curve,
            secret_share,
            new_encryption_key_public,
            protocol_pp,
        )
        .map_err(|e| anyhow!("Re-encryption failed: {}", e))
    }

    pub fn sign(
        &self,
        protocol_pp: Vec<u8>,
        decentralized_party_dkg_output: Vec<u8>,
        user_secret_share: Vec<u8>,
        presign: Vec<u8>,
        message: Vec<u8>,
        signature_algo: u32,
        hash_scheme: u32,
    ) -> Result<Vec<u8>> {
        advance_centralized_sign_party(
            protocol_pp,
            decentralized_party_dkg_output,
            user_secret_share,
            presign,
            message,
            self.curve,
            signature_algo,
            hash_scheme,
        )
        .map_err(|e| anyhow!("Signing failed: {}", e))
    }
}
