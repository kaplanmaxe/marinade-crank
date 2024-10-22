use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::{
    solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSimulateTransactionConfig},
    solana_sdk::signature::Keypair,
};
use anchor_client::{Client, Cluster};
use anchor_lang::prelude::*;
use marinade_client_rs::marinade::{instructions::stake_reserve, rpc_marinade::RpcMarinade};
use solana_sdk::transaction::Transaction;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::read_keypair_file, signer::Signer,
};
use std::{borrow::Borrow, rc::Rc, str::FromStr};

use clap::Parser;

/// CLI tool to turn the marinade crank
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    /// Vote Account to stake
    #[arg(long)]
    vote_account: String,
    /// Path to keypair to use as fee payer
    #[arg(long)]
    keypair: String,
    #[arg(long)]
    simulate: bool,
    #[arg(long)]
    cluster: String,
    #[arg(long = "with-compute-unit-price", required = false)]
    compute_unit_price: Option<u64>,
}

const MARINADE_PROGRAM: &str = "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD";
const STATE_PUBKEY: &str = "8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC";
const RESERVE_BALANCE: &str = "Du3Ysj1wKbxPKkuPPnvzQLQh8oMSVifs3jGZjJWXFmHN";
const VALIDATOR_LIST: &str = "DwFYJNnhLmw19FBTrVaLWZ8SZJpxdPoSYVSJaio9tjbY";

#[allow(clippy::result_large_err)]
fn get_client(
    keypair: &String,
    cluster: &str,
) -> Result<RpcMarinade<Rc<solana_sdk::signature::Keypair>>> {
    let client = Client::new_with_options(
        Cluster::from_str(cluster).unwrap(),
        Rc::new(read_keypair_file(keypair).unwrap()),
        CommitmentConfig::finalized(),
    );
    Ok(
        marinade_client_rs::marinade::rpc_marinade::RpcMarinade::new(
            &client,
            Pubkey::from_str(MARINADE_PROGRAM).unwrap(),
            Pubkey::from_str(STATE_PUBKEY).unwrap(),
        )
        .unwrap(),
    )
}

async fn send_transaction(
    validator_vote: String,
    validator_index: u32,
    keypair: &String,
    simulate: &bool,
    cluster: &String,
    compute_unit_price: &Option<u64>,
) {
    let rpc_url = cluster;

    let wallet = read_keypair_file(keypair).unwrap();
    let client = Client::new_with_options(
        Cluster::from_str(rpc_url).unwrap(),
        Rc::new(wallet),
        CommitmentConfig::finalized(),
    );

    let rpc_client =
        RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::finalized());

    let rpc_marinade_client = marinade_client_rs::marinade::rpc_marinade::RpcMarinade::new(
        &client,
        Pubkey::from_str(MARINADE_PROGRAM).unwrap(),
        Pubkey::from_str(STATE_PUBKEY).unwrap(),
    );
    let stake_account = Keypair::new();
    let program = client.program(Pubkey::from_str(MARINADE_PROGRAM).unwrap());
    let ix = stake_reserve(
        &program,
        &Pubkey::from_str(STATE_PUBKEY).unwrap(),
        &rpc_marinade_client.unwrap().state,
        validator_index,
        &Pubkey::from_str(&validator_vote).unwrap(),
        &stake_account.pubkey(),
        &read_keypair_file(keypair).unwrap().pubkey(),
    )
    .unwrap()
    .instruction(ComputeBudgetInstruction::set_compute_unit_price(
        match compute_unit_price {
            Some(price) => *price,
            None => 0,
        },
    ))
    .instruction(ComputeBudgetInstruction::set_compute_unit_limit(100000))
    .signer(&read_keypair_file(keypair).unwrap())
    .signer(&stake_account)
    .instructions()
    .unwrap();
    let latest_blockhash = rpc_client.get_latest_blockhash().await;
    let tx = Transaction::new_signed_with_payer(
        ix.as_slice(),
        Some(&read_keypair_file(keypair).unwrap().pubkey()),
        &[&read_keypair_file(keypair).unwrap(), &stake_account],
        latest_blockhash.unwrap(),
    );
    if *simulate {
        let result = rpc_client
            .simulate_transaction_with_config(
                &tx,
                RpcSimulateTransactionConfig {
                    sig_verify: true,
                    ..RpcSimulateTransactionConfig::default()
                },
            )
            .await;
        match result {
            Ok(_) => println!("Simulation result: {:?}", result),
            Err(err) => eprintln!("Error: {}", err),
        }
    } else {
        let result = rpc_client
            .send_and_confirm_transaction_with_spinner(&tx)
            .await;
        match result {
            Ok(_) => println!("Transaction signature: {:?}", result),
            Err(err) => eprintln!("Error: {}", err),
        }
    }
}
#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    let client = get_client(&args.keypair, &args.cluster).unwrap();
    let rpc_client =
        RpcClient::new_with_commitment(args.cluster.to_string(), CommitmentConfig::finalized());
    let validator_vote = args.vote_account;
    let mut validator_index = 0;

    match client.borrow().validator_list() {
        Ok((validators, _count)) => {
            for (index, validator) in validators.iter().enumerate() {
                if validator.validator_account.to_string() == validator_vote {
                    let reserve_balance = client
                        .client
                        .get_account(&Pubkey::from_str(RESERVE_BALANCE).unwrap())
                        .unwrap()
                        .lamports;
                    let stake_delta = client.state.stake_delta(reserve_balance);
                    let total_active_balance = client.state.validator_system.total_active_balance;
                    let total_stake_delta: u64;
                    match u64::try_from(stake_delta) {
                        Ok(stake_delta) => {
                            total_stake_delta = stake_delta;
                        }
                        Err(_) => {
                            eprintln!(
                                "Marinade's stake delta is negative ({} SOL), therefore the crank cannot be run this epoch",
                                stake_delta / 1000000000
                            );
                            std::process::exit(1);
                        }
                    }
                    let total_stake_target = total_active_balance.saturating_add(total_stake_delta);
                    let validator_stake_target = client
                        .state
                        .validator_system
                        .validator_stake_target(validator, total_stake_target)
                        .unwrap()
                        / 1000000000;
                    let validator_list_data = rpc_client
                        .get_account_data(&Pubkey::from_str(VALIDATOR_LIST).unwrap())
                        .await;

                    validator_index = index as u32;
                    let validator = client
                        .state
                        .validator_system
                        .get_checked(
                            &validator_list_data.unwrap(),
                            validator_index,
                            &validator.validator_account,
                        )
                        .map_err(|e| e.with_account_name("validator_vote"));
                    let validator_active_balance = validator.unwrap().active_balance / 1000000000;

                    if validator_active_balance >= validator_stake_target {
                        println!("Validator {} already reached stake target. Active balance: {}, stake_target: {}", validator_vote, validator_active_balance, validator_stake_target);
                        std::process::exit(0);
                    }
                    let stake_target = validator_stake_target
                        .saturating_sub(validator_active_balance)
                        .min(total_stake_delta);

                    println!(
                        "Attempting to stake {} with {} SOL",
                        validator_vote,
                        (stake_target as f64)
                    );
                }
            }
        }
        Err(err) => {
            eprintln!("{}", err)
        }
    }
    send_transaction(
        validator_vote,
        validator_index,
        &args.keypair,
        &args.simulate,
        &args.cluster,
        &args.compute_unit_price,
    )
    .await;
}
