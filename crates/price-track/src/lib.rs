mod fourmeme_track;
mod pancake_track;

use alloy::primitives::{Address, address};
use anyhow::Error;
use dashmap::DashMap;
use fourmeme::parser::FourmemeEvent;
use iceoryx2::{node::NodeBuilder, port::server::Server, service::ipc};
use pancake_v2::parser::PancakeSwapEvent;
use rpc::Rpc;
use tokio::sync::mpsc::unbounded_channel;
use tracing::info;
use types::{PriceRequest, PriceResponse};

use crate::{fourmeme_track::FourmemeTrack, pancake_track::PancakeTrack};

const BNB_ADDRESS: Address = address!("0x0000000000000000000000000000000000000000");
pub struct PriceTrack {
    rpc: Rpc,
    tokens: DashMap<Address, u128>, // <token address, wei per token>
    pairs: DashMap<Address, (Address, bool)>, // <pair address, (token address, is_token0)>
    ipc_server: Server<ipc::Service, PriceRequest, (), PriceResponse, ()>,
}

impl PriceTrack {
    /// Initialize PriceTrack
    pub async fn init(rpc: Rpc) -> Result<Self, Error> {
        let node = NodeBuilder::new().create::<ipc::Service>()?;

        let service = node
            .service_builder(&"token_price_query".try_into()?)
            .request_response::<PriceRequest, PriceResponse>()
            .open_or_create()?;

        let ipc_server = service.server_builder().create()?;

        Ok(Self {
            rpc,
            tokens: DashMap::new(),
            pairs: DashMap::new(),
            ipc_server,
        })
    }

    /// Start event listener    
    #[inline]
    pub async fn start(&self) -> Result<(), Error> {
        let (fourmeme_tx, mut fourmeme_rx) = unbounded_channel::<FourmemeEvent>();
        let (pancake_tx, mut pancake_rx) = unbounded_channel::<(PancakeSwapEvent, Address)>();

        let fourmeme_rpc = self.rpc.clone();
        let pancake_rpc = self.rpc.clone();

        // Spawn FourmemeTrack listening task
        tokio::spawn(async move {
            let tracker = FourmemeTrack::new(fourmeme_rpc, fourmeme_tx);
            if let Err(e) = tracker.start().await {
                tracing::error!(?e, "FourmemeTrack error");
            }
        });

        // Spawn PancakeTrack listening task
        tokio::spawn(async move {
            let tracker = PancakeTrack::new(pancake_rpc, pancake_tx);
            if let Err(e) = tracker.start().await {
                tracing::error!(?e, "PancakeTrack error");
            }
        });

        info!("PriceTrack started, waiting for events...");

        loop {
            tokio::select! {
                Some(event) = fourmeme_rx.recv() => {
                    self.handle_fourmeme_event(event);
                }
                Some((event, pair_address)) = pancake_rx.recv() => {
                    self.handle_pancake_event(event, pair_address);
                }
                else => break,
            }

            // SingleThreaded
            if let Err(e) = self.handle_ipc_request() {
                tracing::error!(?e, "IPC request handling error");
            }
        }

        Ok(())
    }

    /// Handle one IPC request (non-blocking check)
    #[inline]
    fn handle_ipc_request(&self) -> Result<(), Error> {
        let ipc_server = &self.ipc_server;

        if let Some(active_request) = ipc_server.receive()? {
            // Extract and convert token address
            let token_address = Address::from(active_request.payload().token_address);
            let price = self.get_token_price(&token_address).unwrap_or(0);

            // build response
            let response = active_request.loan_uninit()?.write_payload(PriceResponse {
                wei_per_token: price,
            });

            response.send()?;
        }

        Ok(())
    }

    #[inline]
    fn handle_fourmeme_event(&self, event: FourmemeEvent) {
        match event {
            FourmemeEvent::TokenPurchase(purchase) => {
                // Purchase event: update token price
                let token = purchase.token;
                let price = purchase.price.to::<u128>();
                self.update_token_price(token, price);
            }
            FourmemeEvent::TokenSale(sale) => {
                // Sale event: update token price
                let token = sale.token;
                let price = sale.price.to::<u128>();
                self.update_token_price(token, price);
            }
            FourmemeEvent::TokenCreate(create) => {
                let token = create.token;
                self.update_token_price(token, 0);
            }
            FourmemeEvent::LiquidityAdded(liquidity) => {
                let other_token = liquidity.quote;
                if other_token != BNB_ADDRESS {
                    self.remove_token(&liquidity.base);
                }
                info!("FourmemeLiquidityAdded: {:?}", liquidity);
            }
        }
    }

    #[inline]
    fn handle_pancake_event(&self, event: PancakeSwapEvent, pair_address: Address) {
        match event {
            PancakeSwapEvent::Sync(sync) => {
                let Some(pair_info) = self.pairs.get(&pair_address) else {
                    return; // Skip if pair not found
                };
                let (token, is_token0) = *pair_info.value();

                let reserve0 = sync.reserve0.to::<u128>();
                let reserve1 = sync.reserve1.to::<u128>();

                // Calculate wei per token
                // If token is token0, price = reserve1 / reserve0 (BNB per token)
                // If token is token1, price = reserve0 / reserve1 (BNB per token)
                let price = if is_token0 {
                    // token is token0, WBNB is token1
                    if reserve0 > 0 {
                        (reserve1 * 1_000_000_000_000_000_000_u128) / reserve0
                    } else {
                        0
                    }
                } else {
                    // token is token1, WBNB is token0
                    if reserve1 > 0 {
                        (reserve0 * 1_000_000_000_000_000_000_u128) / reserve1
                    } else {
                        0
                    }
                };

                self.update_token_price(token, price);
            }
            PancakeSwapEvent::PairCreated(pair_created) => {
                if self.exist_token(&pair_created.token0) {
                    // token0 is our tracked token, token1 is WBNB
                    self.pairs
                        .insert(pair_created.pair, (pair_created.token0, true));
                    return;
                };

                if self.exist_token(&pair_created.token1) {
                    // token1 is our tracked token, token0 is WBNB
                    self.pairs
                        .insert(pair_created.pair, (pair_created.token1, false));
                    return;
                };
            }
            _ => return, // Skip other events
        }
    }

    /// Update token price
    #[inline]
    pub fn update_token_price(&self, token: Address, price: u128) {
        self.tokens.insert(token, price);
    }

    /// Check if token exists    
    #[inline]
    pub fn exist_token(&self, token: &Address) -> bool {
        self.tokens.contains_key(token)
    }

    /// remove token from tokens map    
    #[inline]
    pub fn remove_token(&self, token: &Address) {
        self.tokens.remove(token);
    }

    #[inline]
    pub fn get_token_price(&self, token: &Address) -> Option<u128> {
        self.tokens.get(token).map(|price| *price.value())
    }
}
