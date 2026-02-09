module sovereign_dao::dao {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use std::string::{Self, String};
    use sui::vec_set::{Self, VecSet};
    use sui::event;
    use std::vector;

    // Errors
    const ENotMember: u64 = 0;
    const EProposalNotPassed: u64 = 1;
    const EInvalidProposalId: u64 = 2;
    const ENotTrustedTEE: u64 = 3;
    const EAlreadyVoted: u64 = 4;
    const EProposalExecuted: u64 = 5;
    const EInvalidEncryptionId: u64 = 6;

    // Structs
    public struct DAO has key, store {
        id: UID,
        name: String,
        members: VecSet<address>,
        threshold: u64,
        trusted_tees: VecSet<address>,
        encryption_id: vector<u8>, // ID used for Seal encryption
    }

    public struct Proposal has key, store {
        id: UID,
        dao_id: ID,
        proposer: address,
        title: String,
        description: String,
        votes: VecSet<address>,
        status: u8, // 0: Active, 1: Passed, 2: Failed, 3: Executed
        action_type: u8, // 0: None, 1: AddTEE, 2: RemoveTEE
        action_target: address,
    }

    // Events
    public struct ProposalCreated has copy, drop {
        proposal_id: ID,
        proposer: address,
        action_type: u8,
        action_target: address,
    }

    public struct Voted has copy, drop {
        proposal_id: ID,
        voter: address,
    }

    public struct ProposalExecuted has copy, drop {
        proposal_id: ID,
        executor: address,
    }

    // Functions
    public entry fun create_dao(name: vector<u8>, members: vector<address>, threshold: u64, encryption_id: vector<u8>, ctx: &mut TxContext) {
        let mut members_set = vec_set::empty();
        let mut i = 0;
        while (i < vector::length(&members)) {
            let member = *vector::borrow(&members, i);
            if (!vec_set::contains(&members_set, &member)) {
                vec_set::insert(&mut members_set, member);
            };
            i = i + 1;
        };
        let dao = DAO {
            id: object::new(ctx),
            name: string::utf8(name),
            members: members_set,
            threshold,
            trusted_tees: vec_set::empty(),
            encryption_id,
        };
        transfer::share_object(dao);
    }

    public entry fun create_proposal(dao: &DAO, title: vector<u8>, description: vector<u8>, action_type: u8, action_target: address, ctx: &mut TxContext) {
        let sender = tx_context::sender(ctx);
        assert!(vec_set::contains(&dao.members, &sender), ENotMember);

        let proposal = Proposal {
            id: object::new(ctx),
            dao_id: object::id(dao),
            proposer: sender,
            title: string::utf8(title),
            description: string::utf8(description),
            votes: vec_set::empty(),
            status: 0,
            action_type,
            action_target,
        };
        event::emit(ProposalCreated {
            proposal_id: object::id(&proposal),
            proposer: sender,
            action_type,
            action_target,
        });
        transfer::share_object(proposal);
    }

    public entry fun vote(dao: &DAO, proposal: &mut Proposal, ctx: &mut TxContext) {
        let sender = tx_context::sender(ctx);
        assert!(vec_set::contains(&dao.members, &sender), ENotMember);
        assert!(object::id(dao) == proposal.dao_id, EInvalidProposalId);
        assert!(!vec_set::contains(&proposal.votes, &sender), EAlreadyVoted);
        assert!(proposal.status == 0, EProposalExecuted); // Only vote on Active

        vec_set::insert(&mut proposal.votes, sender);
        event::emit(Voted {
            proposal_id: object::id(proposal),
            voter: sender,
        });

        // Check if passed
        if (vec_set::size(&proposal.votes) >= dao.threshold) {
            proposal.status = 1; // Passed
        }
    }

    public entry fun execute_proposal(dao: &mut DAO, proposal: &mut Proposal, ctx: &mut TxContext) {
        assert!(object::id(dao) == proposal.dao_id, EInvalidProposalId);
        assert!(proposal.status == 1, EProposalNotPassed); // Must be Passed

        let sender = tx_context::sender(ctx);
        // Ideally check permissions (any member can execute?)
        // assert!(vec_set::contains(&dao.members, &sender), ENotMember); // Optional but good.

        // Action Logic
        if (proposal.action_type == 1) { // AddTEE
            if (!vec_set::contains(&dao.trusted_tees, &proposal.action_target)) {
                vec_set::insert(&mut dao.trusted_tees, proposal.action_target);
            };
        } else if (proposal.action_type == 2) { // RemoveTEE
            if (vec_set::contains(&dao.trusted_tees, &proposal.action_target)) {
                vec_set::remove(&mut dao.trusted_tees, &proposal.action_target);
            };
        };

        proposal.status = 3; // Executed
        event::emit(ProposalExecuted {
            proposal_id: object::id(proposal),
            executor: sender,
        });
    }

    // The critical Seal integration function
    // Verified by Seal Key Server to authorize key release.
    // TEE calls this function in a dry-run transaction.
    public entry fun seal_approve(id: vector<u8>, dao: &DAO, proposal: &Proposal, ctx: &TxContext) {
        // 1. Verify id matches DAO encryption ID
        assert!(vectors_equal(&id, &dao.encryption_id), EInvalidEncryptionId);

        // 2. Verify proposal belongs to this DAO
        assert!(object::id(dao) == proposal.dao_id, EInvalidProposalId);

        // 3. Verify proposal is passed OR executed?
        // Seal releases key if proposal Passed.
        // If Executed, it means action taken.
        // TEE usually needs key AFTER passing.
        // If status is 1 (Passed) or 3 (Executed)?
        // Usually strictly Passed. TEE acts on Passed proposals.
        assert!(proposal.status == 1 || proposal.status == 3, EProposalNotPassed);

        // 4. Verify sender is a trusted TEE
        assert!(vec_set::contains(&dao.trusted_tees, &tx_context::sender(ctx)), ENotTrustedTEE);
    }

    // Helpers
    fun vectors_equal(v1: &vector<u8>, v2: &vector<u8>): bool {
        if (vector::length(v1) != vector::length(v2)) { return false };
        let mut i = 0;
        while (i < vector::length(v1)) {
            if (*vector::borrow(v1, i) != *vector::borrow(v2, i)) { return false };
            i = i + 1;
        };
        true
    }

    // Getters
    public fun encryption_id(dao: &DAO): vector<u8> {
        dao.encryption_id
    }

    public fun proposal_status(proposal: &Proposal): u8 {
        proposal.status
    }

    public fun is_trusted_tee(dao: &DAO, addr: address): bool {
        vec_set::contains(&dao.trusted_tees, &addr)
    }

    public fun is_member(dao: &DAO, addr: address): bool {
        vec_set::contains(&dao.members, &addr)
    }
}
