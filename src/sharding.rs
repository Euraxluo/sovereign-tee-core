use anyhow::{Result, anyhow};
use k256::Scalar;
use k256::elliptic_curve::Field;
use rand_core::OsRng;

// Simple Shamir Secret Sharing over Secp256k1 Scalar field

/// Split a secret into N shares, with threshold K
pub fn split_secret(secret: &Scalar, threshold: usize, total: usize) -> Vec<(usize, Scalar)> {
    assert!(threshold <= total);

    // 1. Generate coefficients a_1 ... a_{k-1}
    // a_0 is the secret
    let mut coefficients = Vec::with_capacity(threshold);
    coefficients.push(*secret); // a_0

    for _ in 1..threshold {
        coefficients.push(Scalar::random(&mut OsRng));
    }

    // 2. Evaluate polynomial at x = 1..=total
    let mut shares = Vec::with_capacity(total);
    for x in 1..=total {
        let x_scalar = Scalar::from(x as u64);
        let mut y = Scalar::ZERO;

        // y = a_0 + a_1*x + ... + a_{k-1}*x^{k-1}
        for (i, coeff) in coefficients.iter().enumerate() {
            let x_pow_i = power(&x_scalar, i);
            // k256 Scalar mul takes refs or values depending on version.
            // Based on error, it seems to want values or specific refs.
            // Let's try values.
            y += *coeff * x_pow_i;
        }
        shares.push((x, y));
    }

    shares
}

/// Recover secret from K shares using Lagrange Interpolation
pub fn recover_secret(shares: &[(usize, Scalar)]) -> Result<Scalar> {
    if shares.is_empty() {
        return Err(anyhow!("No shares provided"));
    }

    let mut secret = Scalar::ZERO;

    for (j, (x_j_idx, y_j)) in shares.iter().enumerate() {
        let x_j = Scalar::from(*x_j_idx as u64);

        // Compute Lagrange basis polynomial L_j(0)
        let mut numerator = Scalar::ONE;
        let mut denominator = Scalar::ONE;

        for (m, (x_m_idx, _)) in shares.iter().enumerate() {
            if m == j {
                continue;
            }
            let x_m = Scalar::from(*x_m_idx as u64);

            numerator *= x_m;
            denominator *= x_m - x_j;
        }

        let lagrange_coeff = numerator * denominator.invert().unwrap();
        secret += *y_j * lagrange_coeff;
    }

    Ok(secret)
}

fn power(base: &Scalar, exp: usize) -> Scalar {
    let mut res = Scalar::ONE;
    let mut b = *base;
    let mut e = exp;
    while e > 0 {
        if e % 2 == 1 {
            res *= b;
        }
        b *= b;
        e /= 2;
    }
    res
}
