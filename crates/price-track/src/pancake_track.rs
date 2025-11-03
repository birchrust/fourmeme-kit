use alloy::{
    eips::BlockNumberOrTag,
    primitives::{Address, Log},
    providers::Provider,
    rpc::types::Filter,
};
use anyhow::Error;
use futures_util::StreamExt;
use pancake_v2::{
    constants::{PAIR_CREATED_TOPIC, SWAP_TOPIC, SYNC_TOPIC},
    parser::{PancakeSwapEvent, parse_pancakeswap_event_by_topic},
};
use rpc::Rpc;
use tokio::sync::mpsc;
use tracing::{error, info};

pub struct PancakeTrack {
    rpc: Rpc,
    tx: mpsc::UnboundedSender<(PancakeSwapEvent, Address)>,
}

impl PancakeTrack {
    pub fn new(rpc: Rpc, tx: mpsc::UnboundedSender<(PancakeSwapEvent, Address)>) -> Self {
        Self { rpc, tx }
    }

    pub async fn start(self) -> Result<(), Error> {
        let filter = Filter::new()
            .from_block(BlockNumberOrTag::Latest)
            .event_signature(vec![SWAP_TOPIC, SYNC_TOPIC, PAIR_CREATED_TOPIC]);

        let sub = self.rpc.client.subscribe_logs(&filter).await?;
        let mut stream = sub.into_stream();

        info!("PancakeTrack started listening to events");

        while let Some(log) = stream.next().await {
            // get pair address
            let pair_address = log.address();

            let log = match Log::new(
                log.address(),
                log.topics().to_vec(),
                log.data().data.clone(),
            ) {
                Some(l) => l,
                None => {
                    error!("Failed to create Log");
                    continue;
                }
            };

            // Parse event
            let event = match parse_pancakeswap_event_by_topic(&log) {
                Some(e) => e,
                None => {
                    error!("Failed to parse pancakeswap event");
                    continue;
                }
            };

            // Send event and pair address through channel
            if let Err(e) = self.tx.send((event, pair_address)) {
                error!(?e, "Failed to send event through channel");
                break;
            }
        }

        Ok(())
    }
}
