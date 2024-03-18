use std::error::Error;
use std::str::FromStr;

use bitcoin::{Address, Amount};
use bitcoin::hashes::{Hash, sha256};
use ciborium::into_writer;
use clap::Parser;
use ethers::utils::hex;
use rand::prelude::RngCore;
use thiserror::Error;
use yansi::Paint;

use crate::offer::{OfferRequest, self};
use crate::state::GauloiState;

/// Create the offer for a swap originating from Bitcoin
///
#[derive(Parser, Debug)]
pub struct CreateOfferArgs {
    /// Number of blocks for the BTC to be locked in the Bitcoin HTLC
    #[clap(short, long)]
    lockup_time: Option<u8>,

    /// Amount of BTC to offer
    #[clap(short, long)]
    sell: Option<f64>,

    /// Amount of ETH to receive
    #[clap(short, long)]
    buy: Option<f64>,
}

#[derive(Error, Debug)]
#[error("You don't have enough unspent outputs to create this swap")]
pub struct InsufficientBtcBalanceError {}

pub async fn create_offer(state: &mut GauloiState, args: CreateOfferArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    let p2wpkh = state.p2wpkh_address()?;
    let our_btc_balance = state.btc_api.get_balance(&p2wpkh);
    let editor = &mut state.editor;
    let sold = if let Some(amt) = args.sell {
        amt
    } else {
        let entered = editor.readline("Enter amount of BTC to sell: ")?;
        entered.parse()?
    };

    if let Ok(balance) = our_btc_balance {
        let sold_amt = Amount::from_btc(sold)?;
        let balance_amt = Amount::from_sat(balance as u64);
        if sold_amt > balance_amt {
            println!("We don't have enough BTC to sell this amount! wanted {}, but we have {}",
                Paint::red(sold_amt),
                Paint::red(balance_amt)
            );
            return Ok(());
        }
    }
    let bought = if let Some(amt) = args.buy {
        amt
    } else {
        let entered = editor.readline("Enter amount of ETH to buy: ")?;
        entered.parse()?
    };
    let lockup = if let Some(time) = args.lockup_time {
        time
    } else {
        let entered = editor.readline("Enter lockup time in Bitcoin blocks [default: 10]: ")?;
        if entered.is_empty() {
            10
        } else {
            entered.parse()?
        }
    };
    println!();
    println!("Selling {}BTC for {}ETH", sold, bought);
    println!("Blocks your BTC will be locked up in HTLC for: {}", lockup);

    println!();

    let sold_sats: u128 = bitcoin::Amount::from_btc(sold)?.to_sat() as u128;
    let bought_wei: u128 = ethers::utils::parse_ether(bought)?.as_u128();
    let preimage = GauloiState::preimage();

    let hasher = sha256::Hash::hash(&preimage);

    let offer = OfferRequest {
        version: offer::VERSION,
        sold: sold_sats,
        bought: bought_wei,
        lockup_btc: lockup,
        seller_pubkey_hash: state.btc_address()?.pubkey_hash().to_byte_array(),
        seller_eth_address: state.eth_address()?.to_fixed_bytes(),
        preimage_hash: hasher.to_byte_array(),
    };

    // Add the pending offer nad pre-image to the local storage (full preimage separate)
    state.db.add_pending_offer(offer)?;
    state.db.add_preimage(offer, preimage)?;

    let mut bytes = Vec::new();
    into_writer(&offer, &mut bytes)?;
    println!("hex for offer:\n{}", hex::encode(bytes));
    Ok(())
}