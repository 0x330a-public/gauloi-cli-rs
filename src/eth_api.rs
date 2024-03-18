use std::sync::Arc;

use ethers::contract::ContractCall;
use ethers::contract::FunctionCall;
use ethers::prelude::*;
use ethers::utils::ChainConfig;
use ethers::{
    abi::{Tokenizable, Tokenize},
    contract::abigen,
    prelude::{
        k256::{ecdsa::SigningKey, Secp256k1},
        Address, MiddlewareBuilder, SignerMiddleware,
    },
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::U256,
};
use url::Url;

abigen!(
    GauloiFactory,
    "./src/abi/GauloiFactory.json",
    derives(serde::Deserialize, serde::Serialize),
);

pub struct EthApi {
    pub client: Arc<Provider<Http>>,
    gauloi_address: Address,
}

impl EthApi {
    fn testnet_gauloi() -> Address {
        "0x8c1f0b50D535E0c06B315Ab0d9F18775b98e4CE5"
            .parse::<Address>()
            .unwrap()
    }

    pub fn mainnet_gauloi() -> Address {
        // hypothetically, in a self defence situation
        "0x0E2B0838c33e5cE63101B0FBdf86b011bd1C649D"
            .parse::<Address>()
            .unwrap()
    }

    pub fn testnet() -> Self {
        let client =
            Provider::try_from("https://eth-sepolia.public.blastapi.io".to_string()).unwrap();
        EthApi {
            client: Arc::new(client),
            gauloi_address: EthApi::testnet_gauloi(),
        }
    }

    pub fn mainnet() -> Self {
        let client = Provider::try_from("https://eth.llamarpc.com".to_string()).unwrap();
        EthApi {
            client: Arc::new(client),
            gauloi_address: EthApi::mainnet_gauloi(),
        }
    }

    pub fn new_mainnet(gauloi_address: &str) -> Self {
        let client = Provider::try_from("https://eth.llamarpc.com".to_string()).unwrap();
        EthApi {
            client: Arc::new(client),
            gauloi_address: gauloi_address.parse::<Address>().unwrap(),
        }
    }

    pub async fn get_balance(&self, address: &Address) -> Result<U256, Box<dyn std::error::Error>> {
        let current_balance = self.client.get_balance(address.clone(), None).await?;
        Ok(current_balance)
    }

    pub async fn commit_eth(
        &self,
        signer: Wallet<SigningKey>,
        seller: Address,
        amount: U256,
        preimage_hash: [u8; 32],
        timeout: U256,
    ) -> Result<Option<TransactionReceipt>, anyhow::Error> {
        let from = signer.address();
        let signed_client = self.client.clone().with_signer(signer);
        let gauloi = GauloiFactory::new(self.gauloi_address, Arc::new(signed_client));

        let create_call: ContractCall<_, ()> = gauloi
            .create_swap(seller.clone(), preimage_hash, timeout)
            .value(amount);
        let in_flight = create_call.send().await?;

        let receipt = in_flight.confirmations(1).await?;

        Ok(receipt)
    }

    pub async fn claim_eth(
        &self,
        signer: Wallet<SigningKey>,
        swap_id: U256,
        preimage: [u8; 32],
    ) -> Result<Option<TransactionReceipt>, anyhow::Error> {
        let signed_client = self.client.clone().with_signer(signer);
        let gauloi = GauloiFactory::new(self.gauloi_address, Arc::new(signed_client));

        let claim_call: ContractCall<_, ()> = gauloi.claim_swap(swap_id, preimage);
        let in_flight = claim_call.send().await?;

        let receipt = in_flight.confirmations(1).await?;

        Ok(receipt)
    }

    pub async fn our_swap_id(&self, preimage_hash: [u8; 32]) -> Result<U256, anyhow::Error> {
        let gauloi = GauloiFactory::new(self.gauloi_address, self.client.clone());

        let get_id_call: ContractCall<_, U256> = gauloi.hash_to_swap_map(preimage_hash);
        let swap_id: U256 = get_id_call.call().await?;
        Ok(swap_id)
    }

    pub async fn our_swap_by_id(&self, our_swap_id: U256) -> Result<Swap, anyhow::Error> {
        let gauloi = GauloiFactory::new(self.gauloi_address, self.client.clone());
        let get_swap_call: ContractCall<_, _> = gauloi.swaps(our_swap_id);
        let swap_token = get_swap_call.call().await?.into_token();
        let swap = Swap::from_token(swap_token)?;

        Ok(swap)
    }

    pub async fn our_swap(&self, preimage_hash: [u8; 32]) -> Result<Swap, anyhow::Error> {
        let swap_id = self.our_swap_id(preimage_hash).await?;
        return self.our_swap_by_id(swap_id).await;
    }
}
