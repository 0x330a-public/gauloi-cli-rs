use std::ops::Index;

use bitcoin::hashes::{sha256, Hash};
use clap::Parser;

use crate::state::GauloiState;

/// List the pending offers we have created
#[derive(Parser, Debug)]
pub struct ListArgs {
    
}

pub async fn print_all(state: &mut GauloiState, _args: ListArgs) -> Result<(), anyhow::Error> {
    println!();
    let all_swaps = state.db.get_all_offers()?;
    println!("Total pending offers: {}", all_swaps.len());
    println!();
    all_swaps.iter().enumerate().for_each(|(index, swap)| {
        let mut bytes_writer = Vec::new();
        ciborium::into_writer(&swap, &mut bytes_writer).unwrap();
        println!("[{}] Swap: Buy {}ETH / Sell {}BTC, lockup: {}, preimage-hash: {}, ready to execute? {}",
            index,
            ethers::utils::format_ether(swap.bought),
            bitcoin::Amount::from_sat(swap.sold as u64).to_btc(),
            swap.lockup_btc,
            hex::encode(swap.preimage_hash),
            true
        );
    });
    
    Ok(())
}