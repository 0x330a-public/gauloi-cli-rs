use crate::state::GauloiState;
use serde::Serialize;
use ethers::utils::hex;
use tokio::io::{stdout, AsyncWriteExt};
use yansi::Paint;



pub async fn print_addresses(state: &mut GauloiState, _args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!();
    println!("Bitcoin address:");
    let btc_pub = state.btc_address()?;
    let address = bitcoin::Address::p2wpkh(&btc_pub, state.network)?;
    let btc_balance = state.btc_api.get_balance(&address)?;
    let eth_add = state.eth_address()?;
    let eth_balance_future = state.eth_api.get_balance(&eth_add);
    println!("{}", address);
    let balance_text = "Fetching Unspent balance...";
    print!("{}",balance_text);
    stdout().flush().await?;
    print!("\r{}",balance_text.chars().map(|_| ' ').collect::<String>());
    stdout().flush().await?;
    println!("\rUnspent: {}BTC",Paint::yellow(bitcoin::amount::Amount::from_sat(btc_balance as u64).to_btc()));

    println!();
    println!("Ethereum address:");
    println!("{}", hex::encode_prefixed(eth_add));
    let balance_text = "Fetching ETH balance...";
    print!("{}",balance_text);
    stdout().flush().await?;
    let eth_balance = eth_balance_future.await?;
    print!("\r{}",balance_text.chars().map(|_| ' ').collect::<String>());
    stdout().flush().await?;
    println!("\rBalance: {}ETH",Paint::yellow(ethers::utils::format_ether(eth_balance).trim_end_matches('0')));
    println!();
    Ok(())
}