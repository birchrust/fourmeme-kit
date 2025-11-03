use alloy::primitives::{Address, B256, address, b256};

/// Fourmeme
pub const FOURMEME_CONTRACT: Address = address!("0x5c952063c7fc8610FFDB798152D69F0B9550762b");
/// LiquidityAdded event topic
pub const LIQUIDITY_ADDED_TOPIC: B256 =
    b256!("0xc18aa71171b358b706fe3dd345299685ba21a5316c66ffa9e319268b033c44b0");
pub const TOKEN_PURCHASE_TOPIC: B256 =
    b256!("0x7db52723a3b2cdd6164364b3b766e65e540d7be48ffa89582956d8eaebe62942");
pub const TOKEN_SALE_TOPIC: B256 =
    b256!("0x0a5575b3648bae2210cee56bf33254cc1ddfbc7bf637c0af2ac18b14fb1bae19");
pub const TOKEN_CREATE_TOPIC: B256 =
    b256!("0x396d5e902b675b032348d3d2e9517ee8f0c4a926603fbc075d3d282ff00cad20");
