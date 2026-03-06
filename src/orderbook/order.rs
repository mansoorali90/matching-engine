// Allow dead_code for public API that may be used by external code
#![allow(dead_code)]

use crate::orderbook::side::{OrderType, Side, TimeInForce};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Unique identifier for an order
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrderId(Uuid);

impl OrderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, OrderError> {
        Uuid::parse_str(s)
            .map(OrderId)
            .map_err(|_| OrderError::InvalidOrderId("Invalid UUID format".to_string()))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for OrderId {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur when validating or processing orders
#[derive(Debug, Error, PartialEq, Eq)]
pub enum OrderError {
    #[error("Invalid price: {0}")]
    InvalidPrice(String),
    #[error("Invalid quantity: {0}")]
    InvalidQuantity(String),
    #[error("Invalid order ID: {0}")]
    InvalidOrderId(String),
    #[error("Insufficient balance: required={required}, available={available}")]
    InsufficientBalance { required: u64, available: u64 },
    #[error("Order not found: {0}")]
    OrderNotFound(String),
    #[error("Duplicate order ID: {0}")]
    DuplicateOrderId(String),
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),
    #[error("Invalid side for operation")]
    InvalidSide,
    #[error("Order expired")]
    OrderExpired,
    #[error("Circuit breaker triggered")]
    CircuitBreakerTriggered,
    #[error("Price band violation: price={price}, limit={limit}")]
    PriceBandViolation { price: u64, limit: u64 },
    #[error("Self-trade detected")]
    SelfTrade,
}

/// Represents an order in the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Unique order identifier
    pub order_id: OrderId,
    /// Buy or Sell
    pub side: Side,
    /// Order type (Market, Limit, IOC, FOK)
    pub order_type: OrderType,
    /// Time in force (GTC, IOC, FOK, GTD)
    pub time_in_force: TimeInForce,
    /// Quantity in base currency smallest units (e.g., satoshis)
    pub quantity: u64,
    /// Price in quote currency smallest units (e.g., cents) as Decimal for precision
    pub price: Decimal,
    /// Trading pair symbol (e.g., "BTC/USD")
    pub symbol: String,
    /// Account ID of the order creator
    pub account_id: String,
    /// Timestamp in nanoseconds since epoch
    pub timestamp_ns: u64,
    /// Sequence number for ordering and replay
    pub sequence_number: u64,
    /// Expiration timestamp in nanoseconds (for GTD orders)
    pub expiration_ns: Option<u64>,
    /// Filled quantity so far
    pub filled_quantity: u64,
}

impl Order {
    /// Create a new limit order
    pub fn new_limit(
        side: Side,
        quantity: u64,
        price: Decimal,
        symbol: String,
        account_id: String,
    ) -> Self {
        Self {
            order_id: OrderId::new(),
            side,
            order_type: OrderType::Limit,
            time_in_force: TimeInForce::GTC,
            quantity,
            price,
            symbol,
            account_id,
            timestamp_ns: current_time_ns(),
            sequence_number: 0,
            expiration_ns: None,
            filled_quantity: 0,
        }
    }

    /// Create a new market order
    pub fn new_market(side: Side, quantity: u64, symbol: String, account_id: String) -> Self {
        Self {
            order_id: OrderId::new(),
            side,
            order_type: OrderType::Market,
            time_in_force: TimeInForce::IOC,
            quantity,
            price: Decimal::ZERO,
            symbol,
            account_id,
            timestamp_ns: current_time_ns(),
            sequence_number: 0,
            expiration_ns: None,
            filled_quantity: 0,
        }
    }

    /// Get remaining quantity
    pub fn remaining_quantity(&self) -> u64 {
        self.quantity.saturating_sub(self.filled_quantity)
    }

    /// Check if order is fully filled
    pub fn is_filled(&self) -> bool {
        self.filled_quantity >= self.quantity
    }

    /// Check if order is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiration) = self.expiration_ns {
            current_time_ns() > expiration
        } else {
            false
        }
    }

    /// Validate the order
    pub fn validate(&self, max_price: Decimal, max_quantity: u64) -> Result<(), OrderError> {
        // Validate price
        if self.price < Decimal::ZERO {
            return Err(OrderError::InvalidPrice(
                "Price cannot be negative".to_string(),
            ));
        }
        if self.price > max_price {
            return Err(OrderError::InvalidPrice(format!(
                "Price {} exceeds maximum {}",
                self.price, max_price
            )));
        }

        // Validate quantity
        if self.quantity == 0 {
            return Err(OrderError::InvalidQuantity(
                "Quantity must be greater than 0".to_string(),
            ));
        }
        if self.quantity > max_quantity {
            return Err(OrderError::InvalidQuantity(format!(
                "Quantity {} exceeds maximum {}",
                self.quantity, max_quantity
            )));
        }

        // Validate remaining quantity
        if self.remaining_quantity() == 0 {
            return Err(OrderError::InvalidQuantity(
                "Order already fully filled".to_string(),
            ));
        }

        // Validate order ID
        // (OrderId type ensures validity)

        // Validate not expired
        if self.is_expired() {
            return Err(OrderError::OrderExpired);
        }

        Ok(())
    }

    /// Fill part of the order
    pub fn fill(&mut self, quantity: u64) -> Result<(), OrderError> {
        let new_filled = self
            .filled_quantity
            .checked_add(quantity)
            .ok_or_else(|| OrderError::InvalidQuantity("Quantity overflow".to_string()))?;

        if new_filled > self.quantity {
            return Err(OrderError::InvalidQuantity(
                "Fill quantity exceeds order quantity".to_string(),
            ));
        }

        self.filled_quantity = new_filled;
        Ok(())
    }
}

/// Get current time in nanoseconds since epoch
pub fn current_time_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_order_id_generation() {
        let id1 = OrderId::new();
        let id2 = OrderId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_order_id_from_string() {
        let id = OrderId::new();
        let id_str = id.to_string();
        let parsed = OrderId::from_string(&id_str).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_order_creation() {
        let order = Order::new_limit(
            Side::Buy,
            100,
            dec!(50000.00),
            "BTC/USD".to_string(),
            "account1".to_string(),
        );
        assert_eq!(order.side, Side::Buy);
        assert_eq!(order.quantity, 100);
        assert_eq!(order.price, dec!(50000.00));
        assert_eq!(order.filled_quantity, 0);
    }

    #[test]
    fn test_remaining_quantity() {
        let mut order = Order::new_limit(
            Side::Buy,
            100,
            dec!(50000.00),
            "BTC/USD".to_string(),
            "account1".to_string(),
        );
        assert_eq!(order.remaining_quantity(), 100);

        order.fill(30).unwrap();
        assert_eq!(order.remaining_quantity(), 70);
    }

    #[test]
    fn test_order_validation() {
        let order = Order::new_limit(
            Side::Buy,
            100,
            dec!(50000.00),
            "BTC/USD".to_string(),
            "account1".to_string(),
        );
        assert!(order.validate(dec!(1000000.00), 10000).is_ok());
    }

    #[test]
    fn test_order_validation_invalid_price() {
        let order = Order::new_limit(
            Side::Buy,
            100,
            dec!(50000.00),
            "BTC/USD".to_string(),
            "account1".to_string(),
        );
        assert!(matches!(
            order.validate(dec!(10000.00), 10000),
            Err(OrderError::InvalidPrice(_))
        ));
    }

    #[test]
    fn test_order_fill() {
        let mut order = Order::new_limit(
            Side::Buy,
            100,
            dec!(50000.00),
            "BTC/USD".to_string(),
            "account1".to_string(),
        );

        order.fill(50).unwrap();
        assert_eq!(order.filled_quantity, 50);
        assert!(!order.is_filled());

        order.fill(50).unwrap();
        assert!(order.is_filled());
    }

    #[test]
    fn test_order_fill_overflow() {
        let mut order = Order::new_limit(
            Side::Buy,
            100,
            dec!(50000.00),
            "BTC/USD".to_string(),
            "account1".to_string(),
        );

        assert!(matches!(
            order.fill(150),
            Err(OrderError::InvalidQuantity(_))
        ));
    }
}
