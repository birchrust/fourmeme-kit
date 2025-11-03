use alloy::{primitives::Log, sol, sol_types::SolEvent};

macro_rules! try_parse_event {
    ($log:expr, $event_type:ty, $variant:path) => {
        <$event_type>::decode_log($log.as_ref())
            .ok()
            .map(|event| $variant(event.data))
    };
}

sol! {
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    contract PancakeSwapPair {
        /// Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to)
        event Swap(
            address indexed sender,
            uint256 amount0In,
            uint256 amount1In,
            uint256 amount0Out,
            uint256 amount1Out,
            address indexed to
        );

        /// Sync(uint112 reserve0, uint112 reserve1)
        event Sync(
            uint112 reserve0,
            uint112 reserve1
        );
    }
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    contract PancakeSwapFactory {
        /// PairCreated(address indexed token0, address indexed token1, address pair, uint256)
        event PairCreated(
            address indexed token0,
            address indexed token1,
            address pair,
            uint256
        );
    }
}

pub use PancakeSwapFactory::PairCreated;
pub use PancakeSwapPair::{Swap, Sync};

#[derive(Debug, Clone)]
pub enum PancakeSwapEvent {
    Swap(Swap),
    Sync(Sync),
    PairCreated(PairCreated),
}

#[inline]
pub fn parse_pancakeswap_event_by_topic(log: &Log) -> Option<PancakeSwapEvent> {
    try_parse_event!(log, Swap, PancakeSwapEvent::Swap)
        .or_else(|| try_parse_event!(log, Sync, PancakeSwapEvent::Sync))
        .or_else(|| try_parse_event!(log, PairCreated, PancakeSwapEvent::PairCreated))
}
