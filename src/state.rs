use std::str::FromStr;
use std::sync::Arc;

use bitcoin::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::hashes::{sha256, Hash};
use bitcoin::opcodes::all::*;
use bitcoin::{PublicKey, Network, ScriptBuf, Script, Address, address};
use bitcoin::secp256k1::{All, Secp256k1 as BtcSecp};
use ethers::prelude::*;
use ethers::prelude::k256::{Secp256k1, ecdsa::SigningKey};
use rand::RngCore;
use rand::prelude::ThreadRng;
use rustyline::DefaultEditor;

use crate::bitcoin_api::BitcoinApi;
use crate::eth_api::EthApi;
use crate::swaps::SwapStorage;
use crate::offer::Offer;

pub struct GauloiState {
    pub secp: BtcSecp<All>,
    pub master_extended: ExtendedPrivKey,
    pub editor: DefaultEditor,
    pub network: Network,
    pub btc_api: BitcoinApi,
    pub eth_api: EthApi,
    pub db: SwapStorage,
}

impl GauloiState {
    pub fn btc_derivation() -> Result<DerivationPath, anyhow::Error> {
        let derivation = DerivationPath::from_str("m/84h/0h/1h/0/0")?;
        Ok(derivation)
    }

    pub fn btc_address(&self) -> Result<PublicKey, anyhow::Error> {
        let derivation = &self.master_extended.derive_priv(&self.secp, &GauloiState::btc_derivation()?)?;
        Ok(PublicKey::from_private_key(&self.secp, &derivation.to_priv()))
    }

    pub fn our_pubkey_hash(&self) -> Result<[u8;20], anyhow::Error> {
        let btc_address = self.btc_address()?;
        Ok(btc_address.pubkey_hash().to_byte_array())
    }

    pub fn p2wpkh_address(&self) -> Result<Address, anyhow::Error> {
        let address = self.btc_address()?;
        Ok(Address::p2wpkh(&address, self.network)?)
    }

    pub fn get_wallet(&self) -> Result<Wallet<SigningKey>, anyhow::Error> {
        let signer = SigningKey::from_slice(&self.master_extended.private_key.secret_bytes())?;
        Ok(Wallet::from(signer))
    }

    pub fn eth_address(&self) -> Result<H160, anyhow::Error> {
        let wallet = self.get_wallet()?;
        Ok(wallet.address())
    }

    pub fn preimage() -> [u8;32] {
        let mut rng = ThreadRng::default();
        let mut preimage_bytes = [0u8;32];
        rng.fill_bytes(&mut preimage_bytes);
        preimage_bytes
    }

}
