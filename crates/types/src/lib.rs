use iceoryx2::prelude::ZeroCopySend;

/// query price request
#[derive(Debug, ZeroCopySend)]
#[repr(C)]
pub struct PriceRequest {
    pub token_address: [u8; 20],
}

/// query price response
#[derive(Debug, ZeroCopySend)]
#[repr(C)]
pub struct PriceResponse {
    pub wei_per_token: u128, // price
}
