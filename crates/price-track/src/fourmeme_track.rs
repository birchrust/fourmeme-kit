use alloy::{eips::BlockNumberOrTag, primitives::Log, providers::Provider, rpc::types::Filter};
use anyhow::Error;
use fourmeme::{
    constants::{FOURMEME_CONTRACT, TOKEN_CREATE_TOPIC, TOKEN_PURCHASE_TOPIC, TOKEN_SALE_TOPIC},
    parser::{FourmemeEvent, parse_fourmeme_event_by_topic},
};
use futures_util::StreamExt;
use rpc::Rpc;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct FourmemeTrack {
    rpc: Rpc,
    tx: mpsc::UnboundedSender<FourmemeEvent>,
}

impl FourmemeTrack {
    pub fn new(rpc: Rpc, tx: mpsc::UnboundedSender<FourmemeEvent>) -> Self {
        Self { rpc, tx }
    }

    /// Start listening to Fourmeme events and send through channel
    pub async fn start(self) -> Result<(), Error> {
        let filter = Filter::new()
            .from_block(BlockNumberOrTag::Latest)
            .address(vec![FOURMEME_CONTRACT])
            .event_signature(vec![
                TOKEN_PURCHASE_TOPIC,
                TOKEN_SALE_TOPIC,
                TOKEN_CREATE_TOPIC,
            ]);

        let sub = self.rpc.client.subscribe_logs(&filter).await?;
        let mut stream = sub.into_stream();

        info!("FourmemeTrack started listening to events");

        while let Some(log) = stream.next().await {
            let Some(log) = Log::new(log.address(), log.topics().to_vec(), log.data().data.clone()) else {
                error!("Failed to create Log");
                continue;
            };

            let Some(event) = parse_fourmeme_event_by_topic(&log) else {
                error!("Failed to parse fourmeme event");
                continue;
            };

            // Send event through channel
            if let Err(e) = self.tx.send(event) {
                error!(?e, "Failed to send event through channel");
                break;
            }
        }

        Ok(())
    }
}
