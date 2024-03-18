use bitcoin::hashes::Hash;
use bitcoin::hashes::sha256;
use ciborium::from_reader;
use ciborium::into_writer;
use clap::Parser;
use ethers::types::U256;
use ethers::utils::{hex, format_ether};
use yansi::Paint;

use crate::offer::{OfferRequest, OfferResponse, VERSION};
use crate::state::GauloiState;

/// Parse the response for an offer we initiated
#[derive(Parser, Debug)]
pub struct ParseOfferArgs {
    /// Hex encoded offer
    offer: String,
}

pub async fn parse_offer(
    state: &mut GauloiState,
    args: ParseOfferArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = hex::decode(args.offer)?;
    let offer: OfferRequest = from_reader(bytes.as_slice())?;

    let seller_is_us = offer.seller_pubkey_hash == state.btc_address()?.pubkey_hash().to_byte_array();
    if seller_is_us {
        println!("\nThis is an offer you created!\n");
        return Ok(());
    }
    
    let eth_address = &state.eth_address()?;

    let eth_balance = state.eth_api.get_balance(&eth_address).await?;

    let offer_u256 = U256::from(offer.bought);

    if eth_balance < offer_u256 {
        println!(
            "Insufficient balance to execute this swap, we have {}, but offer is for {}",
            Paint::red(eth_balance),
            Paint::red(offer_u256),
        );
        return Ok(());
    }

    println!();
    println!("{}", Paint::yellow("=== Trade Offer ==="));
    println!("Sell {}ETH to receive {}?", format_ether(offer_u256), bitcoin::Amount::from_sat(offer.sold as u64));
    let input = state.editor.readline("[Y]/n?")?;
    if !input.is_empty() && input.to_lowercase().contains("n") {
        // Exit early
        println!("{}", Paint::red("Not importing trade offer, exiting"));
        return Ok(());
    }

    let entered = state.editor.readline(format!("Enter lockup time in Eth blocks [default: {}]: ", offer.lockup_btc).as_str())?;
    let lockup_eth = if entered.is_empty() {
        offer.lockup_btc
    } else {
        entered.parse()?
    };

    println!("Adding offer...");

    state.db.add_pending_offer(offer)?;

    let request_hash = sha256::Hash::hash(bytes.as_slice());

    let my_pubkey_hash = state.btc_address()?.pubkey_hash();
    let my_eth_address = state.eth_address()?;

    let response = OfferResponse {
        version: VERSION,
        sold: offer.sold,
        bought: offer.bought,
        lockup_eth: lockup_eth,
        buyer_pubkey_hash: my_pubkey_hash.to_byte_array(),
        buyer_eth_address: my_eth_address.to_fixed_bytes(),
        request_hash: request_hash.to_byte_array(),
    };

    state.db.add_offer_response(response)?;

    let mut bytes = Vec::new();
    into_writer(&response, &mut bytes)?;
    println!("hex for response:\n{}", hex::encode(bytes));
    Ok(())
}

