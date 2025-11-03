use anyhow::{Error, Result};
use reqwest::Client;
use serde_json::{Value, json};
pub struct Bloxroute {
    client: Client,
    api_key: String,
    url: String,
}

impl Bloxroute {
    pub fn init(api_key: String) -> Self {
        let url = String::from("https://api.blxrbdn.com");
        let client = Client::new();

        Self {
            client,
            api_key,
            url,
        }
    }

    pub async fn send_private_tx(&self, tx: String) -> Result<Value, Error> {
        // Remove 0x prefix if present (documentation says transaction should be without 0x prefix)
        let tx_hex = if tx.starts_with("0x") || tx.starts_with("0X") {
            tx[2..].to_string()
        } else {
            tx
        };

        let response = self
            .client
            .post(self.url.clone())
            .header("Content-Type", "application/json")
            .header("Authorization", self.api_key.clone())
            .json(&json!({
                "id": "1",
                "jsonrpc": "2.0",
                "method": "bsc_private_tx",
                "params": {
                    "transaction": tx_hex,
                    "mev_builders": ["all"]
                }
            }))
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;

        // Extract txHash from result object: result.result.txHash
        if let Some(result_obj) = result.get("result") {
            return Ok(result_obj.clone());
        }

        // Log the full response for debugging
        let error = result
            .get("error")
            .map(|e| e.to_string())
            .unwrap_or_else(|| format!("Unknown error. Full response: {}", result));
        Err(Error::msg(format!("BloxRoute API error: {}", error)))
    }
}
