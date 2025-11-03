use alloy::primitives::Address;
use anyhow::Error;
use iceoryx2::port::client::Client;
use iceoryx2::{node::NodeBuilder, service::ipc};
use types::{PriceRequest, PriceResponse};

pub struct PriceQuery {
    client: Client<ipc::Service, PriceRequest, (), PriceResponse, ()>,
}

impl PriceQuery {
    pub async fn init() -> Result<Self, Error> {
        let node = NodeBuilder::new().create::<ipc::Service>()?;

        let service = node
            .service_builder(&"token_price_query".try_into()?)
            .request_response::<PriceRequest, PriceResponse>()
            .open_or_create()?;

        let client = service.client_builder().create()?;
        Ok(Self { client })
    }

    #[inline]
    pub async fn query_price(&self, token_address: Address) -> Result<u128, Error> {
        let request = PriceRequest {
            token_address: token_address.0.0,
        };
        let response = self.client.send_copy(request)?;
        match response.receive()? {
            Some(response) => Ok(response.payload().wei_per_token),
            None => Err(Error::msg("No response received")),
        }
    }
}
