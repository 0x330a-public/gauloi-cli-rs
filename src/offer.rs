use bitcoin::{ScriptBuf, Script, opcodes::all::*};
use serde::{Deserialize, Serialize};

pub const VERSION: u8 = 0;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct OfferRequest {
    /// Version the request / response agreement protocol for backwards incompatibility (potentially)
    pub version: u8,
    /// Amount of BTC sold
    pub sold: u128,
    /// Amount of ETH bought
    pub bought: u128,
    /// Lockup time of BTC in HTLC (number of blocks)
    pub lockup_btc: u8,
    /// Our Bitcoin redemption address's HASH_160'd pubkey (RIPEMD_160(SHA256)) for timeout claim
    pub seller_pubkey_hash: [u8; 20],
    /// Our ETH redemption address
    pub seller_eth_address: [u8; 20],
    /// The preimage-hash
    pub preimage_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct OfferResponse {
    /// Version the request / response agreement protocol for backwards incompatibility (potentially)
    pub version: u8,
    /// Amount of BTC sold (sats)
    pub sold: u128,
    /// Amount of ETH bought (wei)
    pub bought: u128,
    /// Blocks that ETH will be locked up for
    pub lockup_eth: u8,
    /// The buyer's pubkey HASH_160'd
    pub buyer_pubkey_hash: [u8; 20],
    /// The buyer's ETH address
    pub buyer_eth_address: [u8; 20],
    /// The SHA256 hash of the offer request's cbor bytes
    pub request_hash: [u8; 32],
}


/// This is mostly for storage on CLI, basically the same as the offer request + response
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// Version the request / response agreement protocol for backwards incompatibility (potentially)
    pub version: u8,
    /// Amount of BTC sold (sats)
    pub sold: u128,
    /// Amount of ETH bought (wei)
    pub bought: u128,
    /// Blocks that ETH will be locked up for
    pub lockup_eth: u8,
    /// Blocks that BTC will be locked up for
    pub lockup_btc: u8,
    /// The seller's pubkey HASH_160'd
    pub seller_pubkey_hash: [u8; 20],
    /// The buyer's pubkey HASH_160'd
    pub buyer_pubkey_hash: [u8; 20],
    /// The seller's ETH address
    pub seller_eth_address: [u8; 20],
    /// The buyer's ETH address
    pub buyer_eth_address: [u8; 20],
    /// The ETH swap committment ID (in SwapFactory)
    pub swap_id_hex: Option<String>,
    /// The preimage-hash
    pub preimage_hash: [u8; 32],
    /// The offer request hash, to use as a "unique" lookup locally for checking pre-images etc
    pub request_hash: [u8;32],
}

impl Offer {

    pub fn is_user_seller(&self, user_pubkey_hash_or_eth_address: [u8;20]) -> bool {
        self.seller_pubkey_hash == user_pubkey_hash_or_eth_address
            || self.seller_eth_address == user_pubkey_hash_or_eth_address
    }

    pub fn is_user_buyer(&self, user_pubkey_hash_or_eth_address: [u8;20]) -> bool {
        self.buyer_pubkey_hash == user_pubkey_hash_or_eth_address
            || self.buyer_eth_address == user_pubkey_hash_or_eth_address
    }

    pub fn htlc_script(&self) -> ScriptBuf {
        Script::builder()
        // claim path
        .push_opcode(OP_IF)
        .push_opcode(OP_SHA256)
        .push_slice(self.preimage_hash)
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(self.buyer_pubkey_hash)
        // else, timeout path
        .push_opcode(OP_ELSE)
        .push_int(self.lockup_btc as i64)
        .push_opcode(OP_CSV)
        .push_opcode(OP_DROP)
        .push_opcode(OP_DUP)
        .push_opcode(OP_HASH160)
        .push_slice(self.seller_pubkey_hash)
        .push_opcode(OP_ENDIF)
        // end if/else
        .push_opcode(OP_EQUALVERIFY)
        .push_opcode(OP_CHECKSIG)
        .into_script()
    }

}


#[derive(Serialize, Deserialize)]
pub struct BtcCommitment {
    /// Transaction ID for the BTC value commitment to a htlc (to look up TX state)
    pub tx_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct EthCommitment {
    /// Hex value of the U256 number (because we can't serde U256 directly?)
    pub swap_id_hex: String,
}

