#[cfg(test)]
mod tests {
    use crate::pss::perform_pss_refresh;
    use crate::sharding::{recover_secret, split_secret};
    use crate::sui_utils::{build_and_hash_sui_tx, pubkey_to_sui_address};
    use dwallet_mpc_types::dwallet_mpc::DWalletCurve;

    use group::secp256k1::scalar::PublicParameters;
    use group::{OsCsRng, Samplable};
    use k256::Scalar;
    use k256::ecdsa::{SigningKey, VerifyingKey};
    use k256::elliptic_curve::Field;
    use k256::elliptic_curve::PrimeField;
    use rand_core::OsRng;

    // Type alias for group::Scalar to avoid confusion with k256::Scalar
    use group::secp256k1::Scalar as GroupScalar;

    // --- PSS Tests ---
    #[test]
    fn test_pss_refresh_math() {
        let pp = PublicParameters::default();
        let s1 = GroupScalar::sample(&pp, &mut OsCsRng).unwrap();
        let s2 = GroupScalar::sample(&pp, &mut OsCsRng).unwrap();

        let s1_bytes = bcs::to_bytes(&s1).unwrap();
        let s2_bytes = bcs::to_bytes(&s2).unwrap();

        let result = perform_pss_refresh(DWalletCurve::Secp256k1 as u32, &s1_bytes, &s2_bytes)
            .expect("PSS refresh failed");

        let s1_new: GroupScalar = bcs::from_bytes(&result.new_dao_share).unwrap();
        let s2_new: GroupScalar = bcs::from_bytes(&result.new_tee_share).unwrap();

        let sum_old = s1 + s2;
        let sum_new = s1_new + s2_new;

        assert_eq!(
            sum_old, sum_new,
            "Sum of shares must remain invariant after refresh"
        );
        assert_ne!(s1, s1_new, "Share 1 must change");
        assert_ne!(s2, s2_new, "Share 2 must change");
    }

    // --- Sharding Tests ---
    #[test]
    fn test_shamir_secret_sharing() {
        let secret = Scalar::random(&mut OsRng);

        // Split into 5 shares, threshold 3
        let shares = split_secret(&secret, 3, 5);
        assert_eq!(shares.len(), 5);

        // Recover with 3 shares (Should succeed)
        let subset_3 = &shares[0..3];
        let recovered_3 = recover_secret(subset_3).expect("Recovery failed with k=3");
        assert_eq!(
            secret, recovered_3,
            "Secret recovered with k shares must match"
        );

        // Recover with 5 shares (Should succeed)
        let recovered_5 = recover_secret(&shares).expect("Recovery failed with k=5");
        assert_eq!(
            secret, recovered_5,
            "Secret recovered with n shares must match"
        );

        // Recover with 2 shares (Should FAIL or produce wrong result)
        let subset_2 = &shares[0..2];
        let recovered_2 = recover_secret(subset_2).expect("Math should run");
        assert_ne!(
            secret, recovered_2,
            "Recovering with < k shares must fail to match secret"
        );
    }

    // --- Sui Utils Tests ---
    #[test]
    fn test_sui_address_generation() {
        // Test Vector: Known Keypair
        // Private Key: 1
        let one = Scalar::ONE;
        let signing_key = SigningKey::from_bytes(&one.to_bytes()).unwrap();
        let verifying_key = VerifyingKey::from(&signing_key);

        let addr = pubkey_to_sui_address(&verifying_key);
        assert!(addr.starts_with("0x"));
        assert_eq!(addr.len(), 66); // 0x + 64 hex chars (32 bytes)
    }

    #[test]
    fn test_sui_tx_hashing() {
        let hash = build_and_hash_sui_tx("0xSender", "0xRecipient", 100).unwrap();
        assert_eq!(hash.len(), 32); // Blake2b-256
    }
}
