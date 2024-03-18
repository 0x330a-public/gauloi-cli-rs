use std::time::Duration;

use anyhow::{bail, Result};
use clap::Parser;
use ethers::types::{H160, U256};
use tokio::time::sleep;

use crate::{offer::Offer, state::GauloiState};

/// Execute a swap by id
#[derive(Parser, Debug)]
pub struct ExecuteArgs {
    swap_id: usize,
}

pub async fn execute(state: &mut GauloiState, args: ExecuteArgs) -> Result<()> {
    let offer = state.db.get_complete_offer(args.swap_id)?;

    println!("=== Found offer, executing swap... ===");
    println!("[1/4] Checking BTC Commit...");
    await_or_start_btc_commitment(state, &offer).await?;
    println!("[2/4] Checking ETH Commit...");
    await_or_start_eth_commitment(state, &offer).await?;
    println!("[3/4] Checking ETH Claim...");
    await_or_claim_eth_commitment(state, &offer).await?;

    // if we need to, claim the btc
    if offer.is_user_buyer(state.our_pubkey_hash()?) {
        println!("[4/4] Claiming BTC...");
        claim_btc_commitment(state, &offer).await?
    } else {
        println!("[4/4] Swap executed successfully");
    }

    Ok(())
}

async fn await_or_start_btc_commitment(state: &GauloiState, offer: &Offer) -> Result<()> {
    let mut attempts = 0;
    let htlc_script = offer.htlc_script();
    let htlc_address = bitcoin::Address::p2wsh(&htlc_script, state.network);
    let our_btc = state.p2wpkh_address()?;

    let fee = 1200;
    loop {
        attempts += 1;
        if attempts > 10 {
            bail!("BTC not committed yet, try again later!");
        }

        let htlc_balance = state.btc_api.get_balance(&htlc_address).unwrap();
        if offer.is_user_seller(state.our_pubkey_hash()?) && htlc_balance < offer.sold {
            let remainder = offer.sold - htlc_balance;
            // plus miner fee?
            let ins = state
                .btc_api
                .find_unspents_for_value(&our_btc, remainder + fee).unwrap();
                let tx = state.btc_api.build_transaction(
                    state,
                    ins,
                    &htlc_address,
                    &our_btc,
                    remainder as u64,
                    fee as u64,
                )?;
            println!("Committing BTC...");
            state.btc_api.submit_tx(tx).unwrap();
            break;
        } else {
            println!("Looking for BTC commitment...");
            if htlc_balance >= offer.sold {
                break;
            }
        }
        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}

async fn await_or_start_eth_commitment(state: &mut GauloiState, offer: &Offer) -> Result<()> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        if attempts > 10 {
            break;
        }

        if let Ok(swap) = state.eth_api.our_swap(offer.preimage_hash).await {
            if swap.value == U256::from(offer.bought) {
                break;
            } else if swap.preimage_hash == offer.preimage_hash {
                bail!("swap commitment doesn't have the bought ETH amount")
            }
        }

        if offer.is_user_buyer(state.our_pubkey_hash()?) {
            // we have to commit
            let timeout = offer.lockup_eth;
            let signer = state.get_wallet()?;
            let seller = H160::from_slice(offer.seller_eth_address.as_slice());
            let amount = offer.bought;
            let preimage_hash = offer.preimage_hash;
            println!("Committing ETH...");
            state.eth_api.commit_eth(signer, seller, U256::from(amount), preimage_hash, U256::from(timeout)).await?;
            break;
        } else {
            println!("Looking for ETH Commitment...");
        }
        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}

async fn await_or_claim_eth_commitment(state: &mut GauloiState, offer: &Offer) -> Result<()> {
    let mut attempts = 0;
    loop {
        attempts += 1;
        if attempts > 10 {
            break;
        }

        let swap_id = state.eth_api.our_swap_id(offer.preimage_hash).await?;
        let swap = state.eth_api.our_swap_by_id(swap_id).await?;

        // preimage is 32 bytes of 0 if not claimed by reveal
        if swap.preimage != [0u8;32] {
            break;
        }

        if offer.is_user_seller(state.our_pubkey_hash()?) {
            // we have to commit
            let signer = state.get_wallet()?;
            let preimage_opt = state.db.get_preimage(offer.request_hash.as_slice())?;
            if let Some(preimage) = preimage_opt {
                println!("Claiming ETH via preimage...");
                state.eth_api.claim_eth(signer, swap_id, preimage).await?;
                break;
            } else {
                bail!("No preimage for this swap!")
            }
        } else {
            println!("Looking for ETH claim via preimage...");
        }
        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}

async fn claim_btc_commitment(state: &mut GauloiState, offer: &Offer) -> Result<()> {
    let mut attempts = 0;
    let htlc_script = offer.htlc_script();
    let htlc_address = bitcoin::Address::p2wsh(&htlc_script, state.network);
    let our_btc = state.p2wpkh_address()?;

    let swap = state.eth_api.our_swap(offer.preimage_hash).await?;
    if swap.preimage == [0u8;32] { // expect this is already here
        bail!("Preimage isn't committed yet!")
    }

    let fee = 1200;
    loop {
        attempts += 1;
        if attempts > 10 {
            bail!("BTC couldn't be claimed!");
        }


        let htlc_balance = state.btc_api.get_balance(&htlc_address).unwrap();
        if offer.is_user_buyer(state.our_pubkey_hash()?) {
            println!("Claiming BTC...");
            // plus miner fee?
            let ins = state
                .btc_api
                .get_utxos(&htlc_address).unwrap();
            let tx = state.btc_api.build_claim_btc(
                state,
                ins,
                &our_btc,
                htlc_script.clone(),
                swap.preimage,
                fee as u64,
            )?;
            state.btc_api.submit_tx(tx).unwrap();
            break;
        }
        sleep(Duration::from_secs(10)).await;
    }
    Ok(())
}
