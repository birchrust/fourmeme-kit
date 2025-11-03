use alloy::hex;
use alloy::providers::Provider; // bring Provider trait into scope for methods like get_gas_price
use alloy::{primitives::Address, signers::local::PrivateKeySigner};
use alloy::{
    providers::{DynProvider, IpcConnect, ProviderBuilder, WsConnect},
    transports::http::reqwest::Url,
};
use anyhow::{Context, Error, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// The type of connection to use for the RPC client.
#[derive(Debug)]
pub enum ConnectType {
    Http(String),
    Ws(String),
    Ipc(String),
}

/// Thin wrapper around an Alloy provider configured with runtime chain data.
#[derive(Clone, Debug)]
pub struct Rpc {
    pub client: DynProvider,
    pub sender_address: Address,
    pub gas_price: Arc<Mutex<u128>>,
    pub nonce: Arc<Mutex<u64>>,
}

impl Rpc {
    pub async fn init(connect_type: ConnectType, private_key: &str) -> Result<Self, Error> {
        let signer: PrivateKeySigner = private_key.parse()?;
        let sender_address = signer.address();
        let provider = match connect_type {
            ConnectType::Http(url) => ProviderBuilder::new()
                .wallet(signer)
                .connect_http(url.parse::<Url>().with_context(|| "Invalid HTTP URL")?),
            ConnectType::Ws(url) => ProviderBuilder::new()
                .wallet(signer)
                .connect_ws(WsConnect::new(&url))
                .await
                .with_context(|| format!("Failed to connect to WebSocket"))?,
            ConnectType::Ipc(path) => ProviderBuilder::new()
                .wallet(signer)
                .connect_ipc(IpcConnect::new(path))
                .await
                .with_context(|| format!("Failed to connect to IPC"))?,
        };

        let gas_price = Arc::new(Mutex::new(1000000000));
        let nonce = Arc::new(Mutex::new(0));

        Ok(Self {
            client: DynProvider::new(provider),
            sender_address,
            gas_price,
            nonce,
        })
    }

    /// Get the current gas price
    #[inline]
    pub async fn get_gas_price(&self) -> u128 {
        *self.gas_price.lock().await
    }

    /// Get the current nonce from cache
    #[inline]
    pub async fn get_nonce(&self) -> u64 {
        *self.nonce.lock().await
    }

    /// Increment and get the next nonce (for sending multiple transactions)
    #[inline]
    pub async fn get_and_increment_nonce(&self) -> u64 {
        let mut nonce = self.nonce.lock().await;
        let current = *nonce;
        *nonce += 1;
        current
    }

    /// Send a raw signed transaction to the network
    ///
    /// # Arguments
    ///
    /// * `tx_hex` - The raw transaction hex string (with or without 0x prefix)
    ///
    /// # Returns
    ///
    /// * `Result<String, Error>` - The transaction hash on success
    ///
    /// # Note
    ///
    /// This method sends a pre-signed transaction directly to the RPC node.
    /// The transaction should already be signed and encoded as a hex string.
    #[inline]
    pub async fn send_raw_transaction(&self, tx_hex: String) -> Result<String, Error> {
        // Remove 0x prefix if present
        let tx_hex_clean = if tx_hex.starts_with("0x") || tx_hex.starts_with("0X") {
            &tx_hex[2..]
        } else {
            &tx_hex
        };

        // Decode hex string to bytes
        let tx_bytes = hex::decode(tx_hex_clean)
            .with_context(|| format!("Failed to decode transaction hex: {}", tx_hex))?;

        // Send the raw transaction
        let pending_tx = self
            .client
            .send_raw_transaction(&tx_bytes)
            .await
            .with_context(|| "Failed to send raw transaction")?;

        // Get the transaction hash
        let tx_hash = format!("{:?}", pending_tx.tx_hash());

        Ok(tx_hash)
    }

    #[inline]
    pub async fn update_gas_price(
        &self,
        interval: Duration,
        saturation: u128,
    ) -> Result<JoinHandle<()>, Error> {
        // Spawn a background task to refresh gas price periodically
        let client_cloned = self.client.clone();
        let gas_price_cloned: Arc<Mutex<u128>> = Arc::clone(&self.gas_price);
        let task = tokio::spawn(async move {
            loop {
                match client_cloned.get_gas_price().await {
                    Ok(price) => {
                        let mut lock = gas_price_cloned.lock().await;
                        *lock = price.saturating_mul(saturation);
                    }
                    Err(e) => {
                        eprintln!("failed to fetch gas price: {:?}", e);
                    }
                }
                // update gas price every 5 seconds
                sleep(interval).await;
            }
        });

        Ok(task)
    }

    /// Start a background task to update nonce periodically
    ///
    /// # Arguments
    ///
    /// * `interval` - The interval between updates
    ///
    /// # Returns
    ///
    /// * `JoinHandle` - The handle to the background task
    #[inline]
    pub async fn update_nonce(&self, interval: Duration) -> Result<JoinHandle<()>, Error> {
        // Spawn a background task to refresh nonce periodically
        let client_cloned = self.client.clone();
        let sender_address = self.sender_address;
        let nonce_cloned: Arc<Mutex<u64>> = Arc::clone(&self.nonce);
        let task = tokio::spawn(async move {
            loop {
                match client_cloned.get_transaction_count(sender_address).await {
                    Ok(count) => {
                        let mut lock = nonce_cloned.lock().await;
                        // Only update if the on-chain nonce is higher (prevents going backwards)
                        if count > *lock {
                            *lock = count;
                        }
                    }
                    Err(e) => {
                        eprintln!("failed to fetch nonce: {:?}", e);
                    }
                }
                sleep(interval).await;
            }
        });

        Ok(task)
    }
}
