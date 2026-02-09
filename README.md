# Sovereign DAO Gateway Core

> **"Address never changes, control always shifts."**
> (地址永驻，控制流转)

This crate (`sovereign-tee-core`) is the reference implementation of a **TEE-based 2PC-MPC Vault** designed for the Sui Network. It enables a DAO to hold assets (on Sui, BTC, or ETH) under a single, immutable address while dynamically rotating the actual control permissions (private key shares) among members using Proactive Secret Sharing (PSS) and Trusted Execution Environments (TEE).

---

## 1. The Blueprint (原始愿景)

### Project Goal
To build an automated DAO execution system where the **on-chain address remains permanent**, but the **control rights are dynamic**. By stripping the core cryptography from Ika and running it inside a private **TEE (SGX/TDX)**, combined with Sui's native privacy primitives (**Seal + Walrus**), we achieve a Sovereign Vault that resists both physical extraction and centralized censorship.

### System Architecture
1.  **Anchor**: The DAO holds a fixed aggregated public key $P$ (Secp256k1).
2.  **Sharding**: The private key $sk$ is split into $s_{DAO}$ (DAO side) and $s_{TEE}$ (TEE side).
    *   $s_{DAO}$: Encrypted by **Seal** and stored on **Walrus**.
    *   $s_{TEE}$: Protected by TEE hardware (Stateless / Sealed).
3.  **Dynamic Resharing (PSS)**: When members change, TEE triggers the PSS protocol:
    *   $s_{DAO}' = s_{DAO} + \alpha$
    *   $s_{TEE}' = s_{TEE} - \alpha$
    *   **Result**: The public key $P$ remains unchanged, but old shares are mathematically invalidated.
4.  **Decentralized Control**: Only when a Sui on-chain proposal passes, the Seal contract releases the decryption key for $s_{DAO}$ to the TEE.

---

## 2. Core Findings (核心发现)

During the investigation and implementation of `dwallet` and `inkrypto` libraries, we uncovered several critical insights that shaped this architecture:

### A. The "Re-encryption" Trap
*   **Initial Discovery**: The native Ika client used "Re-encryption" for share rotation. This meant the underlying mathematical value of the share didn't change, only the wrapper key did.
*   **The Sovereign Gap**: If an attacker possessed the old TEE key and the old encrypted share, they could still recover the key.
*   **Our Solution**: We switched to **True Proactive Secret Sharing (PSS)**. By applying complementary mathematical perturbations ($\pm \alpha$) to both shares, we ensure that **old backups become mathematically useless**, regardless of encryption status.

### B. Stateless TEE Architecture
*   **Challenge**: "Holding" a share inside TEE persistent storage creates a single point of failure (physical theft of the specific CPU).
*   **Insight**: TEE should be a **Stateless Functional Unit**.
    *   It should not store long-term secrets.
    *   It should receive encrypted shares ($s_{DAO}$ from Seal, $s_{TEE}$ from TEE-Sealed Blob), compute the signature, and immediately **wipe its memory**.
*   **Result**: This allows for elastic TEE scaling and disaster recovery.

### C. NFT Sharding (DeFi Governance)
*   **Innovation**: Instead of treating $s_{DAO}$ as a single file, we integrated **Shamir Secret Sharing (SSS)**.
*   **Mechanism**: $s_{DAO}$ is split into $N$ fragments, each minted as a **Sui NFT**.
*   **Impact**: Governance becomes tradeable. To execute a transaction, the TEE acts as an aggregator, collecting enough NFT-backed shards to reconstruct $s_{DAO}$ in memory.

---

## 3. Implementation Strategies (实现策略)

The CLI tool supports two distinct governance strategies:

### Strategy A: Standard Seal (The "Boardroom" Model)
*   **Logic**: $s_{DAO}$ is a single encrypted file protected by a Sui Smart Contract (Seal).
*   **Flow**: Members Vote -> Threshold Reached -> Seal Decrypts -> TEE Signs.
*   **Best For**: Stable DAOs, Corporate Treasuries.

### Strategy B: NFT Sharding (The "DeFi" Model)
*   **Logic**: $s_{DAO}$ is mathematically shattered into $N$ pieces via $(t, n)$-threshold sharing.
*   **Flow**: Users hold NFT Shards -> Submit Shards to TEE -> TEE Interpolates Secret -> TEE Signs.
*   **Best For**: Liquid DAOs, Community-owned Vaults.

---

## 4. Codebase Structure (项目结构)

Located at `crates/sovereign-tee-core/`:

| File | Description |
| :--- | :--- |
| **`src/main.rs`** | The CLI entrypoint. Orchestrates the full lifecycle: `Genesis`, `Launch`, `Execute`, `Refresh`. Handles strategy selection. |
| **`src/pss.rs`** | **Core Math Engine**. Implements the Proactive Secret Sharing logic ($s \pm \alpha$) and ECDSA signature simulation. |
| **`src/sharding.rs`** | **Shamir Engine**. Implements Lagrange Interpolation over the `k256` scalar field for NFT sharding. |
| **`src/sui_utils.rs`** | **Sui Native Integration**. Generates Secp256k1 Sui Addresses and computes Blake2b-256 Transaction Digests. |
| **`src/tee_service.rs`** | Abstraction layer for TEE operations (DKG, Decryption, Signing). |
| **`src/scalar_utils.rs`** | Utilities for converting between raw bytes (BCS) and cryptographic scalars (k256). |
| **`src/dao.rs`** | Data structures for simulating DAO membership, keys, and voting logic. |
| **`e2e_test.sh`** | Automated script demonstrating the full lifecycle of both strategies. |

## 5. Usage

```bash
# Build
cargo build -p sovereign-tee-core

# Run Automated Demo
./e2e_test.sh

# Manual CLI
cargo run -p sovereign-tee-core -- --help
```
# sovereign-tee-core
