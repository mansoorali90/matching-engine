// Shared test utilities
use crate::orderbook::{FeeConfig, OrderBook, OrderBookConfig};
use rust_decimal_macros::dec;

/// Helper function to create a test order book without circuit breaker
pub fn create_test_orderbook(symbol: &str) -> OrderBook {
    let config = OrderBookConfig {
        max_price: dec!(1_000_000.00),
        max_quantity: 10_000_000,
        fee_config: FeeConfig::new(10, 20),
        circuit_breaker_config: None, // No circuit breaker
        enable_self_trade_prevention: true,
    };
    OrderBook::with_config(symbol.to_string(), config, None)
}
