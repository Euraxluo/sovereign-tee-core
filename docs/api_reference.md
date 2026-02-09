# Sovereign DAO Contract API Reference

Module: `sovereign_dao::dao`

## Structs

### `DAO`
The main DAO object, stored as a shared object on-chain.
*   `id`: `UID` - Unique Identifier.
*   `name`: `String` - DAO name.
*   `members`: `VecSet<address>` - Set of members.
*   `threshold`: `u64` - Number of votes required to pass a proposal.
*   `trusted_tees`: `VecSet<address>` - Set of whitelisted TEE addresses.
*   `encryption_id`: `vector<u8>` - The Seal identity ID used to encrypt the private key share.

### `Proposal`
A governance proposal.
*   `id`: `UID` - Unique Identifier.
*   `dao_id`: `ID` - ID of the parent DAO.
*   `proposer`: `address` - Address of the proposer.
*   `title`: `String` - Proposal title.
*   `description`: `String` - Proposal description.
*   `votes`: `VecSet<address>` - Set of voters.
*   `status`: `u8` - 0: Active, 1: Passed, 2: Failed, 3: Executed.
*   `action_type`: `u8` - 0: None, 1: AddTEE, 2: RemoveTEE.
*   `action_target`: `address` - The target address for the action (e.g., TEE to add/remove).

## Functions

### `create_dao`
Creates a new DAO.
*   **Signature**: `public entry fun create_dao(name: vector<u8>, members: vector<address>, threshold: u64, encryption_id: vector<u8>, ctx: &mut TxContext)`
*   **Access**: Public.

### `create_proposal`
Creates a new proposal within a DAO.
*   **Signature**: `public entry fun create_proposal(dao: &DAO, title: vector<u8>, description: vector<u8>, action_type: u8, action_target: address, ctx: &mut TxContext)`
*   **Access**: Members only.

### `vote`
Votes on an active proposal.
*   **Signature**: `public entry fun vote(dao: &DAO, proposal: &mut Proposal, ctx: &mut TxContext)`
*   **Access**: Members only.
*   **Effect**: Updates vote count. If threshold reached, updates status to `Passed`.

### `execute_proposal`
Executes a passed proposal, applying governance actions.
*   **Signature**: `public entry fun execute_proposal(dao: &mut DAO, proposal: &mut Proposal, ctx: &mut TxContext)`
*   **Access**: Public (permissionless execution, logic handles checks).
*   **Effect**: If `AddTEE`, adds TEE to whitelist. Sets status to `Executed`.

### `seal_approve`
Verifies authorization for Seal to release the decryption key.
*   **Signature**: `public entry fun seal_approve(id: vector<u8>, dao: &DAO, proposal: &Proposal, ctx: &TxContext)`
*   **Logic**:
    *   Verifies `id` matches DAO's `encryption_id`.
    *   Verifies `proposal` belongs to DAO.
    *   Verifies `proposal.status == Passed` or `Executed`.
    *   Verifies `ctx.sender()` is a trusted TEE.
*   **Usage**: Called by Seal Key Server during `fetch_key`.
