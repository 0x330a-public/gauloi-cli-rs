use std::path::Path;

use rand::RngCore;
use tokio::fs::{read,write};
use bitcoin::{Network};
use bitcoin::bip32::ExtendedPrivKey;
use bitcoin::secp256k1::{Secp256k1};
use bitcoin_api::BitcoinApi;
use eth_api::EthApi;
use rustyline::DefaultEditor;
use shellfish::{clap_command, Shell, Command, async_fn};
use swaps::SwapStorage;
use rand::prelude::ThreadRng;

use create::CreateOfferArgs;
use handler::GauloiAsyncHandler;

use list::ListArgs;
use parse::ParseOfferArgs;
use state::GauloiState;
use import::ImportOfferArgs;
use execute::ExecuteArgs;

pub mod create;
pub mod handler;
pub mod offer;
pub mod state;
pub mod parse;
pub mod address;
pub mod bitcoin_api;
pub mod eth_api;
pub mod import;
pub mod swaps;
pub mod list;
pub mod execute;

pub const NETWORK: Network = Network::Bitcoin;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let secp = Secp256k1::new();

    let key_path = Path::new("hotwallet.key");

    if !key_path.exists() {
        let mut rng = ThreadRng::default();
        let mut seed: [u8;32] = [0u8;32];
        rng.fill_bytes(&mut seed);
        let extended = ExtendedPrivKey::new_master(NETWORK, &seed)?;
        write(key_path, extended.encode()).await?;
    }
    let editor = DefaultEditor::new()?;

    let extended = {
        let key = read(key_path).await?;
        ExtendedPrivKey::decode(key.as_slice())?
    };

    let our_state = GauloiState {
        editor,
        secp,
        master_extended: extended,
        network: NETWORK,
        btc_api: BitcoinApi::default(),
        eth_api: EthApi::mainnet(),
        db: SwapStorage::default(),
    };

    let mut shell = Shell::new_with_async_handler(
        our_state,
        "gauloi-cli$ ",
        GauloiAsyncHandler,
        DefaultEditor::new()?,
    );

    shell.commands.insert(
        "address",
        Command::new_async("Print the addresses for this client".to_string(), async_fn!(GauloiState, address::print_addresses))
    );

    shell.commands.insert(
        "create",
        clap_command!(GauloiState, CreateOfferArgs, async create::create_offer),
    );

    shell.commands.insert(
        "parse",
        clap_command!(GauloiState, ParseOfferArgs, async parse::parse_offer),
    );

    shell.commands.insert(
        "list",
        clap_command!(GauloiState, ListArgs, async list::print_all),
    );

    shell.commands.insert(
        "import",
        clap_command!(GauloiState, ImportOfferArgs, async import::import_offer_response),
    );

    shell.commands.insert(
        "execute",
        clap_command!(GauloiState, ExecuteArgs, async execute::execute),
    );

    shell.run_async().await?;

    Ok(())
}
