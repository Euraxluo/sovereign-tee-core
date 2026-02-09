use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use k256::Scalar;
use k256::ecdsa::signature::Signer;
use k256::ecdsa::{Signature, SigningKey, VerifyingKey};
use k256::elliptic_curve::PrimeField; // Trait required for from_repr
use sovereign_tee_core::dao::{DaoGroup, Member};
use sovereign_tee_core::pss::{generate_initial_shares, mock_sign_and_verify, perform_pss_refresh};
use sovereign_tee_core::scalar_utils::bytes_to_scalar;
use sovereign_tee_core::sharding::{recover_secret, split_secret};
use sovereign_tee_core::sui_utils::{build_and_hash_sui_tx, pubkey_to_sui_address};
use std::collections::HashMap;
use std::fs;

#[derive(Parser)]
#[command(name = "sovereign-cli")]
#[command(about = "Sovereign DAO Gateway Management Tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Debug)]
enum Strategy {
    Seal,
    NftSharding,
}

#[derive(Subcommand)]
enum Commands {
    GenesisInit {
        #[arg(long, default_value = "group.json")]
        out: String,
        #[arg(long, default_value = "2")]
        threshold: usize,
    },
    GenesisJoin {
        #[arg(long, default_value = "group.json")]
        group_file: String,
        #[arg(long)]
        name: String,
    },
    GenesisLaunch {
        #[arg(long, default_value = "group.json")]
        group_file: String,
        #[arg(long, default_value = "dao_share.seal")]
        dao_out: String,
        #[arg(long, default_value = "tee_share.store")]
        tee_out: String,
        #[arg(long, value_enum, default_value_t = Strategy::Seal)]
        strategy: Strategy,
        #[arg(long, default_value = "5")]
        shards: usize,
    },
    GroupAddMember {
        #[arg(long, default_value = "group.json")]
        group_file: String,
        #[arg(long)]
        name: String,
    },
    ProposalExecute {
        #[arg(long, default_value = "group.json")]
        group_file: String,
        #[arg(long, default_value = "dao_share.seal")]
        dao_in: String,
        #[arg(long, default_value = "tee_share.store")]
        tee_in: String,
        #[arg(long)]
        recipient: String,
        #[arg(long)]
        amount: u64,
        #[arg(long, value_enum, default_value_t = Strategy::Seal)]
        strategy: Strategy,
        #[arg(long, value_delimiter = ' ', num_args = 1..)]
        shards_in: Option<Vec<String>>,
    },
    GroupRefresh {
        #[arg(long, default_value = "group.json")]
        group_in: String,
        #[arg(long, default_value = "dao_share.seal")]
        dao_in: String,
        #[arg(long, default_value = "tee_share.store")]
        tee_in: String,
        #[arg(long, default_value = "dao_share_new.seal")]
        dao_out: String,
        #[arg(long, default_value = "tee_share_new.store")]
        tee_out: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GenesisInit { out, threshold } => {
            let group = DaoGroup {
                threshold,
                members: Vec::new(),
            };
            let json = serde_json::to_string_pretty(&group)?;
            fs::write(&out, json)?;
            println!(
                "Genesis: Initialized empty group at '{}' with threshold {}",
                out, threshold
            );
        }

        Commands::GenesisJoin { group_file, name } => {
            let content = fs::read_to_string(&group_file)?;
            let mut group: DaoGroup = serde_json::from_str(&content)?;
            if group.members.iter().any(|m| m.name == name) {
                return Err(anyhow!("Member '{}' already exists", name));
            }
            let member = Member::new(&name);
            println!(
                "Member Joined: {} (PubKey: {})",
                member.name, member.pubkey_hex
            );
            group.members.push(member);
            fs::write(&group_file, serde_json::to_string_pretty(&group)?)?;
            println!("Group updated. Total members: {}", group.members.len());
        }

        Commands::GenesisLaunch {
            group_file,
            dao_out,
            tee_out,
            strategy,
            shards,
        } => {
            let content = fs::read_to_string(&group_file)?;
            let group: DaoGroup = serde_json::from_str(&content)?;
            if group.members.len() < group.threshold {
                return Err(anyhow!("Not enough members to launch!"));
            }

            println!("--- Launch Sequence Initiated ---");
            let (s_dao, s_tee) = generate_initial_shares()?;
            let (_, pubkey_hex) = mock_sign_and_verify(&s_dao, &s_tee, b"init")?;
            let pubkey_bytes = hex::decode(&pubkey_hex)?;
            let verifying_key = VerifyingKey::from_sec1_bytes(&pubkey_bytes)?;
            let sui_addr = pubkey_to_sui_address(&verifying_key);

            println!("3. DAO Sui Address Generated: {}", sui_addr);

            match strategy {
                Strategy::Seal => {
                    println!("4. Sealing DAO Share to Walrus...");
                    fs::write(&dao_out, hex::encode(&s_dao))?;
                }
                Strategy::NftSharding => {
                    println!(
                        "4. Sharding DAO Share into {} NFT Blobs (Threshold: {})...",
                        shards, group.threshold
                    );
                    let s_dao_scalar = bytes_to_scalar(&s_dao)?;
                    let shares = split_secret(&s_dao_scalar, group.threshold, shards);

                    for (idx, share) in shares {
                        let filename = format!("shard_{}.hex", idx);
                        fs::write(&filename, hex::encode(share.to_bytes()))?;
                        println!("   -> Minted NFT #{} linked to {}", idx, filename);
                    }
                }
            }

            println!("5. Storing TEE Share...");
            fs::write(&tee_out, hex::encode(&s_tee))?;
            println!("--- Launch Complete: Sovereign DAO is Live ---");
        }

        Commands::GroupAddMember { group_file, name } => {
            let content = fs::read_to_string(&group_file)?;
            let mut group: DaoGroup = serde_json::from_str(&content)?;
            let member = Member::new(&name);
            group.members.push(member);
            fs::write(&group_file, serde_json::to_string_pretty(&group)?)?;
            println!("Added member {}", name);
        }

        Commands::ProposalExecute {
            group_file,
            dao_in,
            tee_in,
            recipient,
            amount,
            strategy,
            shards_in,
        } => {
            let content = fs::read_to_string(&group_file)?;
            let group: DaoGroup = serde_json::from_str(&content)?;

            // 1. Load s_tee as Scalar
            let s_tee_bytes = hex::decode(fs::read_to_string(&tee_in)?.trim())?;
            let s_tee_scalar = bytes_to_scalar(&s_tee_bytes)?;

            // 2. Load s_dao as Scalar
            let s_dao_scalar = match strategy {
                Strategy::Seal => {
                    let bytes = hex::decode(fs::read_to_string(&dao_in)?.trim())?;
                    bytes_to_scalar(&bytes)?
                }
                Strategy::NftSharding => {
                    let files =
                        shards_in.ok_or(anyhow!("Strategy NftSharding requires --shards-in"))?;
                    if files.len() < group.threshold {
                        return Err(anyhow!(
                            "Not enough shards! Need {}, got {}",
                            group.threshold,
                            files.len()
                        ));
                    }
                    println!("[TEE] Collecting shards from NFT holders...");
                    let mut shares = Vec::new();
                    for file in files {
                        let bytes = hex::decode(fs::read_to_string(&file)?.trim())?;
                        let idx_str = file.replace("shard_", "").replace(".hex", "");
                        let idx: usize = idx_str.parse()?;

                        let scalar_opt = Scalar::from_repr(*k256::FieldBytes::from_slice(&bytes));
                        if scalar_opt.is_none().into() {
                            return Err(anyhow!("Invalid scalar in shard {}", file));
                        }
                        shares.push((idx, scalar_opt.unwrap()));
                        println!("   -> Loaded shard from {}", file);
                    }
                    println!("[TEE] Interpolating Secret from {} shards...", shares.len());
                    recover_secret(&shares)?
                }
            };

            // 3. Reconstruct Private Key & Sign
            let sk = s_dao_scalar + s_tee_scalar;
            let signing_key = SigningKey::from_bytes(&sk.to_bytes())?;
            let verifying_key = VerifyingKey::from(&signing_key);
            let sender = pubkey_to_sui_address(&verifying_key);

            println!(
                "--- Proposal: Transfer {} MIST to {} ---",
                amount, recipient
            );
            println!("Sender (DAO Vault): {}", sender);

            let tx_hash = build_and_hash_sui_tx(&sender, &recipient, amount)?;
            println!("Transaction Digest: {}", hex::encode(&tx_hash));

            if let Strategy::Seal = strategy {
                let mut signatures = HashMap::new();
                println!("Collecting Votes...");
                for i in 0..group.threshold {
                    if i >= group.members.len() {
                        break;
                    }
                    let member = &group.members[i];
                    let sig = member.sign(&tx_hash)?;
                    signatures.insert(member.name.clone(), sig);
                }

                println!("\n[Seal Smart Contract] Verifying signatures...");
                if !group.verify_proposal(&tx_hash, &signatures)? {
                    return Err(anyhow!("Proposal Rejected."));
                }
                println!("[Seal Smart Contract] Access Granted.");
            }

            println!("\n[TEE] Signing transaction digest...");
            let signature: Signature = signing_key.sign(&tx_hash);

            println!("--- Execution Successful ---");
            println!("Signature: {}", hex::encode(signature.to_bytes()));
            println!("Status: VALID SIGNATURE FOR SUI NETWORK");
        }

        Commands::GroupRefresh { .. } => {
            println!("Refresh not fully adapted for NftSharding in this demo.");
        }
    }

    Ok(())
}
