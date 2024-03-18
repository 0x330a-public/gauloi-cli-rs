use anyhow::{bail, Result};
use bitcoin::hashes::{Hash, sha256};
use redb::{Database, ReadableTable, TableDefinition};

use crate::offer::{self, *};

const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("swap_data");
const PREIMAGE_TABLE: TableDefinition<&[u8], &[u8;32]> = TableDefinition::new("preimages");
const OFFER_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("full_swaps");

pub struct SwapStorage {
    db: Database,
}

impl Default for SwapStorage {
    fn default() -> Self {
        let db = Database::create("swaps.db").expect("Couldn't create swap storage");
        SwapStorage {
            db
        }
    }
}

impl SwapStorage {
    
    pub fn get_all_offer_requests(&self) -> Result<Vec<OfferRequest>, anyhow::Error> {
        let read_tx = self.db.begin_read()?;
        let table = read_tx.open_table(TABLE)?;
        let range = table.iter()?;
        let serialized: Vec<OfferRequest> = range.map(|next|{
            let (_, v) = next.unwrap();
            ciborium::from_reader(v.value()).unwrap()
        }).collect();
        
        Ok(serialized)
    }
    
    pub fn get_all_offers(&self) -> Result<Vec<Offer>, anyhow::Error> {
        let read_tx = self.db.begin_read()?;
        let table = read_tx.open_table(OFFER_TABLE)?;
        let range = table.iter()?;
        let serialized: Vec<Offer> = range.map(|next| {
            let (_, v) = next.unwrap();
            ciborium::from_reader(v.value()).unwrap()
        }).collect();

        Ok(serialized)
    }

    pub fn get_swap_index(&self, offer: &Offer) -> Result<Option<usize>, anyhow::Error> {
        let offers = self.get_all_offers()?;
        let found = offers.iter().cloned().enumerate().find(|(_i, o)| offer == o);
        if let Some((i, _)) = found {
            return Ok(Some(i));
        }
        return Ok(None);
    }

    pub fn get_complete_offer(&self, index: usize) -> Result<Offer> {
        if let Some(offer) = self.get_all_offers()?.get(index) {
            Ok(offer.clone())
        } else {
            bail!("No offer locally")
        }
    }

    pub fn add_pending_offer(&self, offer: OfferRequest) -> Result<(), anyhow::Error> {
        let mut writer: Vec<u8> = Vec::new();
        ciborium::into_writer(&offer, &mut writer)?;
        let offer_hasher = sha256::Hash::hash(writer.as_slice());
        let hash = offer_hasher.to_byte_array();
        let write_tx = self.db.begin_write()?;
        {
            let mut write_table = write_tx.open_table(TABLE)?;
            write_table.insert(hash.as_slice(), writer.as_slice())?;
        }
        write_tx.commit()?;
        Ok(())
    }
    
    pub fn get_pending_offer(&self, offer_hash: &[u8]) -> Result<Option<OfferRequest>, anyhow::Error> {
        let read_tx = self.db.begin_read()?;
        let table = read_tx.open_table(TABLE)?;
        
        let offer_query = table.get(offer_hash)?;
        
        if let Some(offer_bytes) = offer_query {
            let results = offer_bytes.value();
            let offer_request = ciborium::from_reader(results)?;
            Ok(Some(offer_request))
        } else {
            Ok(None)
        }
    }
    
    pub fn add_preimage(&self, offer: OfferRequest, preimage: [u8;32]) -> Result<(), anyhow::Error> {
        let mut writer: Vec<u8> = Vec::new();
        ciborium::into_writer(&offer, &mut writer)?;
        let offer_hasher = sha256::Hash::hash(writer.as_slice());
        let hash = offer_hasher.to_byte_array();
        let write_tx = self.db.begin_write()?;
        {
            let mut write_table = write_tx.open_table(PREIMAGE_TABLE)?;
            write_table.insert(hash.as_slice(), &preimage)?;
        }
        write_tx.commit()?;
        
        Ok(())
    }
    
    pub fn get_preimage(&self, offer_hash: &[u8]) -> Result<Option<[u8;32]>, anyhow::Error> {
        let read_tx = self.db.begin_read()?;
        let table = read_tx.open_table(PREIMAGE_TABLE)?;

        let preimage_query = table.get(offer_hash)?;

        if let Some(preimage_bytes) = preimage_query {
            let results = *preimage_bytes.value();
            Ok(Some(results))
        } else {
            Ok(None)
        }
    }
    
    pub fn add_offer_response(&self, response: OfferResponse) -> Result<Offer, anyhow::Error> {

        let pending_offer = self.get_pending_offer(response.request_hash.as_slice())?;
        if let Some(request) = pending_offer {
            let complete_offer = Offer {
                version: offer::VERSION,
                sold: request.sold,
                bought: request.bought,
                lockup_eth: response.lockup_eth,
                lockup_btc: request.lockup_btc,
                seller_pubkey_hash: request.seller_pubkey_hash,
                buyer_pubkey_hash: response.buyer_pubkey_hash,
                seller_eth_address: request.seller_eth_address,
                buyer_eth_address: response.buyer_eth_address,
                swap_id_hex: None,
                request_hash: response.request_hash,
                preimage_hash: request.preimage_hash,
            };

            // cbor the full offer
            let mut writer = Vec::new();
            ciborium::into_writer(&complete_offer, &mut writer)?;

            let write_tx = self.db.begin_write()?;
            {
                let mut write_table = write_tx.open_table(OFFER_TABLE)?;
                write_table.insert(response.request_hash.as_slice(), writer.as_slice())?;
            }
            write_tx.commit()?;
            Ok(complete_offer)
        } else {
            // No offer / request locally
            bail!("No offer / request locally")
        }
    }
}