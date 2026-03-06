use crate::orderbook::order::OrderId;
use crate::orderbook::side::Side;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a trade
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradeId(Uuid);

impl TradeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for TradeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents an executed trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    /// Unique trade identifier
    pub trade_id: TradeId,
    /// Execution price in quote currency smallest units
    pub price: Decimal,
    /// Executed quantity in base currency smallest units
    pub quantity: u64,
    /// Buyer's order ID
    pub buyer_order_id: OrderId,
    /// Seller's order ID
    pub seller_order_id: OrderId,
    /// Buyer's account ID
    pub buyer_account_id: String,
    /// Seller's account ID
    pub seller_account_id: String,
    /// Timestamp in nanoseconds since epoch
    pub timestamp_ns: u64,
    /// Trading pair symbol
    pub symbol: String,
    /// Maker side (provides liquidity)
    pub maker_side: Side,
    /// Taker side (takes liquidity)
    pub taker_side: Side,
    /// Maker fee in quote currency smallest units
    pub maker_fee: Decimal,
    /// Taker fee in quote currency smallest units
    pub taker_fee: Decimal,
    /// Trade value (price * quantity)
    pub trade_value: Decimal,
}

impl Trade {
    pub fn new(
        price: Decimal,
        quantity: u64,
        buyer_order_id: OrderId,
        seller_order_id: OrderId,
        buyer_account_id: String,
        seller_account_id: String,
        symbol: String,
        maker_side: Side,
        maker_fee: Decimal,
        taker_fee: Decimal,
    ) -> Self {
        let trade_value = price * Decimal::from(quantity);
        Self {
            trade_id: TradeId::new(),
            price,
            quantity,
            buyer_order_id,
            seller_order_id,
            buyer_account_id,
            seller_account_id,
            timestamp_ns: crate::orderbook::order::current_time_ns(),
            symbol,
            maker_side,
            taker_side: if maker_side == Side::Buy {
                Side::Sell
            } else {
                Side::Buy
            },
            maker_fee,
            taker_fee,
            trade_value,
        }
    }
}

/// Fee configuration for the matching engine
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FeeConfig {
    /// Maker fee in basis points (1/100th of 1%)
    pub maker_fee_bps: u32,
    /// Taker fee in basis points
    pub taker_fee_bps: u32,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            maker_fee_bps: 10, // 0.10%
            taker_fee_bps: 20, // 0.20%
        }
    }
}

impl FeeConfig {
    pub fn new(maker_fee_bps: u32, taker_fee_bps: u32) -> Self {
        Self {
            maker_fee_bps,
            taker_fee_bps,
        }
    }

    /// Calculate maker fee for a trade
    pub fn calculate_maker_fee(&self, price: Decimal, quantity: u64) -> Decimal {
        let trade_value = price * Decimal::from(quantity);
        trade_value * Decimal::from(self.maker_fee_bps) / Decimal::from(10_000)
    }

    /// Calculate taker fee for a trade
    pub fn calculate_taker_fee(&self, price: Decimal, quantity: u64) -> Decimal {
        let trade_value = price * Decimal::from(quantity);
        trade_value * Decimal::from(self.taker_fee_bps) / Decimal::from(10_000)
    }

    /// Calculate fees for a trade
    pub fn calculate_fees(&self, price: Decimal, quantity: u64) -> (Decimal, Decimal) {
        (
            self.calculate_maker_fee(price, quantity),
            self.calculate_taker_fee(price, quantity),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_trade_id_generation() {
        let id1 = TradeId::new();
        let id2 = TradeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_trade_creation() {
        let trade = Trade::new(
            dec!(50000.00),
            100,
            OrderId::new(),
            OrderId::new(),
            "buyer1".to_string(),
            "seller1".to_string(),
            "BTC/USD".to_string(),
            Side::Buy,
            dec!(5.00),
            dec!(10.00),
        );

        assert_eq!(trade.price, dec!(50000.00));
        assert_eq!(trade.quantity, 100);
        assert_eq!(trade.trade_value, dec!(5000000.00));
        assert_eq!(trade.maker_fee, dec!(5.00));
        assert_eq!(trade.taker_fee, dec!(10.00));
    }

    #[test]
    fn test_fee_calculation() {
        let fee_config = FeeConfig::new(10, 20); // 0.10% maker, 0.20% taker

        let (maker_fee, taker_fee) = fee_config.calculate_fees(dec!(50000.00), 100);

        // 50000 * 100 = 5,000,000
        // Maker fee: 5,000,000 * 0.001 = 5,000
        // Taker fee: 5,000,000 * 0.002 = 10,000
        assert_eq!(maker_fee, dec!(5000.00));
        assert_eq!(taker_fee, dec!(10000.00));
    }

    #[test]
    fn test_fee_calculation_zero() {
        let fee_config = FeeConfig::new(0, 0); // Zero fees

        let (maker_fee, taker_fee) = fee_config.calculate_fees(dec!(50000.00), 100);

        assert_eq!(maker_fee, Decimal::ZERO);
        assert_eq!(taker_fee, Decimal::ZERO);
    }
}
