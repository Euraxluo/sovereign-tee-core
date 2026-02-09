# Sovereign DAO Contract Layer

This document describes the on-chain contract layer for the Sovereign DAO system, implemented in Sui Move.

## Architecture

The system consists of three main components:
1.  **Sovereign DAO Contract** (This implementation): Manages DAO membership, proposals, and voting. It acts as the "Gatekeeper" for the TEE.
2.  **Seal Service**: Provides decentralized secrets management. It holds the encrypted key shares ($s_{DAO}$) and releases them only upon authorization.
3.  **TEE Runner (Nautilus)**: A stateless Trusted Execution Environment that fetches the key from Seal, performs the signing operation, and terminates.

### Workflow

1.  **Setup**:
    *   The DAO is initialized with a set of members and a threshold.
    *   The DAO's private key share ($s_{DAO}$) is encrypted using Seal with a specific identity ID (e.g., the DAO's unique ID).
    *   The DAO whitelists trusted TEE addresses (runners) via governance.

2.  **Governance (Add TEE)**:
    *   A member creates a proposal with `action_type = 1` (Add TEE) and `action_target = TEE_ADDRESS`.
    *   Members vote.
    *   Once threshold is reached, `execute_proposal` is called.
    *   The TEE address is added to the DAO's whitelist.

3.  **Operation (Sign with TEE)**:
    *   A member creates a generic proposal (`action_type = 0`) to authorize a signing operation (e.g., "Sign Tx 123").
    *   Members vote.
    *   Once threshold is reached, status becomes `Passed`.
    *   **Seal Integration**: The TEE Runner detects the passed proposal.
    *   The TEE constructs a transaction calling `seal_approve`.
    *   **Seal Integration**: The Seal Key Server simulates this transaction.
    *   `seal_approve` verifies:
        *   The proposal has `Passed`.
        *   The `id` requested matches the DAO's encryption ID.
        *   The transaction sender (TEE) is a whitelisted runner.
    *   If successful, Seal releases the decryption key to the TEE.
    *   The TEE decrypts $s_{DAO}$, signs the payload, and broadcasts the signature.

## Integration with Seal

The core integration point is the `seal_approve` function in `sovereign_dao::dao`.

```move
public entry fun seal_approve(id: vector<u8>, dao: &DAO, proposal: &Proposal, ctx: &TxContext)
```

This function is designed to be called by the TEE during the Seal `fetch_key` process. It enforces the policy that **keys are only released for passed proposals to trusted TEEs**.

## Integration with Nautilus

The contract maintains a `trusted_tees` list in the `DAO` object.
*   **Registration**: TEEs must be registered via `execute_proposal` (governance).
*   **Authorization**: The `seal_approve` function checks `tx_context::sender(ctx)` against this list.
*   **Statelessness**: The TEE does not need to store state; it relies on the on-chain `Proposal` and `DAO` objects to prove its authorization.
