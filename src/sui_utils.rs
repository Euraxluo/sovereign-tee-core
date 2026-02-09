use anyhow::{Result, anyhow};
use fastcrypto::hash::{Blake2b256, HashFunction};
use k256::ecdsa::VerifyingKey;
use k256::elliptic_curve::group::GroupEncoding;

/// Converts our MPC Secp256k1 Public Key to a real Sui Address
pub fn pubkey_to_sui_address(verifying_key: &VerifyingKey) -> String {
    let pubkey_bytes = verifying_key.to_encoded_point(true).as_bytes().to_vec();

    let mut data = vec![0x01]; // Secp256k1 Flag
    data.extend_from_slice(&pubkey_bytes);

    let hash = Blake2b256::digest(&data);

    // Sui Address is first 32 bytes of Blake2b256(flag || pubkey)
    format!("0x{}", hex::encode(&hash.digest[0..32]))
}

/// Builds a mock Sui Transaction Digest
/// Since we can't easily import `sui-types` due to dependency complexity,
/// we simulate the BCS serialization of a TransferSui transaction.
pub fn build_and_hash_sui_tx(sender: &str, recipient: &str, amount: u64) -> Result<Vec<u8>> {
    // 1. Construct Mock Transaction Data (BCS-like structure)
    let mut mock_tx_data = Vec::new();
    mock_tx_data.extend_from_slice(b"SUI_TX_V1");
    mock_tx_data.extend_from_slice(sender.as_bytes());
    mock_tx_data.extend_from_slice(recipient.as_bytes());
    mock_tx_data.extend_from_slice(&amount.to_le_bytes());

    // 2. Intent Message: Intent(3 bytes) || TransactionData
    // Scope: TransactionData(0), Version: V0(0), App: Sui(0) => [0, 0, 0]
    let mut intent_msg = vec![0u8, 0u8, 0u8];
    intent_msg.extend(mock_tx_data);

    // 3. Hash: Blake2b256(IntentMessage)
    let hash = Blake2b256::digest(&intent_msg);

    Ok(hash.digest.to_vec())
}
