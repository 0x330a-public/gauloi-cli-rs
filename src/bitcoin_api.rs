use std::{collections::BTreeMap, str::FromStr};

use anyhow::bail;
use bitcoin::{
    bip32::{DerivationPath, ExtendedPubKey},
    consensus::Decodable,
    hashes::sha256d,
    locktime::absolute::LockTime,
    psbt::{Input, Psbt, PsbtSighashType},
    secp256k1::Secp256k1,
    transaction, Address, Network, OutPoint, PublicKey, ScriptBuf, Transaction, TxIn, TxOut, Txid,
    Witness,
};
use esplora_api::blocking::ApiClient;
use serde::Deserialize;

use crate::state::GauloiState;

#[derive(Clone)]
pub struct RelevantTxInfo {
    pub txid: Txid,
    pub spend_index: u16,
    pub vout: Vout,
}

pub struct BitcoinApi {
    legacy_client: ApiClient,
}

impl Default for BitcoinApi {
    fn default() -> Self {
        BitcoinApi {
            legacy_client: ApiClient::new("https://blockstream.info/api/", None).unwrap(),
        }
    }
}

#[derive(Clone)]
pub struct Vout {
    value: u64,
    scriptpubkey: ScriptBuf,
}

impl BitcoinApi {
    pub fn get_utxos(
        &self,
        address: &Address,
    ) -> Result<Vec<RelevantTxInfo>, Box<dyn std::error::Error>> {
        let utxos = self
            .legacy_client
            .get_address_utxo(address.to_string().as_str())?;
        let mut relevant = Vec::new();
        for utxo in utxos {
            relevant.push(RelevantTxInfo {
                txid: Txid::from_str(utxo.txid.as_str())?,
                spend_index: utxo.vout,
                vout: Vout {
                    value: utxo.value as u64,
                    scriptpubkey: address.script_pubkey(),
                },
            });
        }
        Ok(relevant)
    }

    pub fn build_transaction(
        &self,
        state: &GauloiState,
        ins: Vec<RelevantTxInfo>,
        to: &Address,
        change_address: &Address,
        value: u64,
        miner_fee: u64,
    ) -> Result<Transaction, anyhow::Error> {
        let secp = Secp256k1::new();
        let xpub = ExtendedPubKey::from_priv(&secp, &state.master_extended);
        let pk = state.btc_address()?;
        let derivation_path = GauloiState::btc_derivation()?;

        let mut inputs = Vec::new();

        let mut accumulated_spend = 0;

        let mut bip32_derivation = BTreeMap::new();
        bip32_derivation.insert(pk.inner, (xpub.fingerprint(), derivation_path.clone()));

        for input in ins.clone() {
            inputs.push(TxIn {
                previous_output: OutPoint {
                    txid: input.txid,
                    vout: input.spend_index as u32,
                },
                ..Default::default()
            });
            accumulated_spend += input.vout.value;
        }

        let change = accumulated_spend - value;
        let unsigned = Transaction {
            input: inputs,
            lock_time: LockTime::ZERO,
            version: 1,
            output: vec![
                TxOut {
                    value,
                    script_pubkey: to.script_pubkey(),
                },
                TxOut {
                    value: change - miner_fee,
                    script_pubkey: change_address.script_pubkey(),
                },
            ],
        };

        let mut psbt = Psbt::from_unsigned_tx(unsigned)?;
        let ty = PsbtSighashType::from_str("SIGHASH_ALL").unwrap();

        psbt.inputs = ins
            .iter()
            .cloned()
            .map(|input| Input {
                witness_utxo: Some(TxOut {
                    value: input.vout.value,
                    script_pubkey: change_address.script_pubkey(),
                }),
                bip32_derivation: bip32_derivation.clone(),
                sighash_type: Some(ty),
                ..Default::default()
            })
            .collect();

        psbt.sign(&state.master_extended, &state.secp).unwrap();
        psbt.inputs.iter_mut().for_each(|input| {
            let sigs: Vec<_> = input.partial_sigs.values().collect();
            let mut script_witness = Witness::new();
            script_witness.push(sigs[0].to_vec());
            script_witness.push(pk.to_bytes());
            input.final_script_witness = Some(script_witness);
            input.partial_sigs = BTreeMap::new();
            input.sighash_type = None;
            input.redeem_script = None;
            input.witness_script = None;
            input.bip32_derivation = BTreeMap::new();
        });
        let extracted = psbt.extract_tx();

        Ok(extracted)
    }

    pub fn build_claim_btc(
        &self,
        state: &GauloiState,
        ins: Vec<RelevantTxInfo>,
        to: &Address,
        htlc_script: ScriptBuf,
        preimage: [u8;32],
        miner_fee: u64,
    ) -> Result<Transaction, anyhow::Error> {
        let secp = Secp256k1::new();
        let xpub = ExtendedPubKey::from_priv(&secp, &state.master_extended);
        let pk = state.btc_address()?;
        let derivation_path = GauloiState::btc_derivation()?;

        let mut inputs = Vec::new();

        let mut value = 0;

        let mut bip32_derivation = BTreeMap::new();
        bip32_derivation.insert(pk.inner, (xpub.fingerprint(), derivation_path.clone()));

        for input in ins.clone() {
            inputs.push(TxIn {
                previous_output: OutPoint {
                    txid: input.txid,
                    vout: input.spend_index as u32,
                },
                ..Default::default()
            });
            value += input.vout.value;
        }
        let unsigned = Transaction {
            input: inputs,
            lock_time: LockTime::ZERO,
            version: 1,
            output: vec![
                TxOut {
                    value: value - miner_fee,
                    script_pubkey: to.script_pubkey(),
                },
            ],
        };

        let mut psbt = Psbt::from_unsigned_tx(unsigned)?;
        let ty = PsbtSighashType::from_str("SIGHASH_ALL").unwrap();

        psbt.inputs = ins
            .iter()
            .cloned()
            .map(|input| Input {
                witness_utxo: Some(TxOut {
                    value: input.vout.value,
                    script_pubkey: htlc_script.clone().to_v0_p2wsh(),
                }),
                bip32_derivation: bip32_derivation.clone(),
                sighash_type: Some(ty),
                witness_script: Some(htlc_script.clone()),
                ..Default::default()
            })
            .collect();

        psbt.sign(&state.master_extended, &state.secp).unwrap();
        psbt.inputs.iter_mut().for_each(|input| {
            let sigs: Vec<_> = input.partial_sigs.values().collect();
            let mut script_witness = Witness::new();
            script_witness.push(sigs[0].to_vec());
            script_witness.push(pk.to_bytes());
            script_witness.push(preimage);
            script_witness.push([1]);
            script_witness.push(htlc_script.clone());
            input.final_script_witness = Some(script_witness);
            input.partial_sigs = BTreeMap::new();
            input.sighash_type = None;
            input.redeem_script = None;
            input.witness_script = None;
            input.bip32_derivation = BTreeMap::new();
        });
        let extracted = psbt.extract_tx();

        Ok(extracted)
    }

    pub fn find_unspents_for_value(
        &self,
        address: &Address,
        value: u128,
    ) -> Result<Vec<RelevantTxInfo>, Box<dyn std::error::Error>> {
        let mut fold_value = 0;
        let mut unspents = Vec::new();
        let utxos = self.get_utxos(address)?;
        for utxo in utxos {
            fold_value += utxo.vout.value as u128;
            unspents.push(utxo);
            if fold_value >= value {
                break;
            }
        }
        Ok(unspents)
    }

    pub fn submit_tx(&self, transaction: Transaction) -> Result<(), Box<dyn std::error::Error>> {
        let hex = bitcoin::consensus::encode::serialize_hex(&transaction);
        let txid = self.legacy_client.post_tx(hex.as_str()).unwrap();
        println!("Submitted: {}", txid);
        Ok(())
    }

    pub fn get_balance(&self, address: &Address) -> Result<u128, Box<dyn std::error::Error>> {
        let balance: u128 = self
            .get_utxos(&address)?
            .iter()
            .fold(0, |amt, utxo| amt + utxo.vout.value as u128);
        Ok(balance)
    }
}
