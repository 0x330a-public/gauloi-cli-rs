use ciborium::from_reader;
use clap::Parser;
use hex;

use crate::offer::*;
use crate::state::GauloiState;

/// Import an offer response
#[derive(Parser, Debug)]
pub struct ImportOfferArgs {
    /// Hex encoded offer response
    offer: String,
}

pub async fn import_offer_response(
    state: &mut GauloiState,
    args: ImportOfferArgs) -> Result<(), anyhow::Error> {
    println!();

    let bytes = hex::decode(args.offer)?;
    let offer_response: OfferResponse = from_reader(bytes.as_slice())?;

    let complete_offer = state.db.add_offer_response(offer_response)?;
    let index = state.db.get_swap_index(&complete_offer)?.unwrap();
    println!("Offer imported successfully!");
    println!("Swap index: {}", index);

    Ok(())
}



