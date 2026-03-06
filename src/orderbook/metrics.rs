// Allow dead_code for public API that may be used by external code
#![allow(dead_code)]

use crate::orderbook::order::Order;
use crate::orderbook::side::Side;
use crate::orderbook::trade::Trade;
use metrics::{counter, gauge, histogram};
use serde::{Deserialize, Serialize};

/// Order book events for logging and streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum OrderBookEvent {
    OrderAdded {
        order: Order,
    },
    OrderCancelled {
        order_id: String,
        reason: String,
    },
    OrderUpdated {
        order_id: String,
        old_quantity: u64,
        new_quantity: u64,
    },
    TradeExecuted {
        trade: Trade,
    },
    PriceLevelChanged {
        side: Side,
        price: String,
        depth: u64,
        delta: i64,
    },
    CircuitBreakerTriggered {
        symbol: String,
        reference_price: String,
    },
    CircuitBreakerReset {
        symbol: String,
    },
}

/// Metrics collector for the order book
#[derive(Debug)]
pub struct OrderBookMetrics {
    symbol: String,
}

impl OrderBookMetrics {
    pub fn new(symbol: &str) -> Self {
        Self {
            symbol: symbol.to_string(),
        }
    }

    /// Record an order received
    pub fn record_order_received(&self, side: Side) {
        let side_str = match side {
            Side::Buy => "bid",
            Side::Sell => "ask",
        };
        counter!("orders_received_total", "side" => side_str.to_string(), "symbol" => self.symbol.clone())
            .increment(1);
    }

    /// Record an order matched
    pub fn record_order_matched(&self, quantity: u64) {
        counter!("orders_matched_total", "symbol" => self.symbol.clone()).increment(1);
        counter!("volume_matched_total", "symbol" => self.symbol.clone()).increment(quantity);
    }

    /// Record an order cancelled
    pub fn record_order_cancelled(&self, side: Side) {
        let side_str = match side {
            Side::Buy => "bid",
            Side::Sell => "ask",
        };
        counter!("orders_cancelled_total", "side" => side_str.to_string(), "symbol" => self.symbol.clone())
            .increment(1);
    }

    /// Record a trade executed
    pub fn record_trade(&self, trade: &Trade) {
        counter!("trades_executed_total", "symbol" => self.symbol.clone()).increment(1);

        let trade_value = trade.trade_value.to_string();
        counter!("trade_volume_total", "symbol" => self.symbol.clone())
            .absolute(trade_value.parse::<u64>().unwrap_or(0));

        histogram!("trade_latency_ns", "symbol" => self.symbol.clone())
            .record(trade.timestamp_ns as f64);

        counter!("fees_collected_maker", "symbol" => self.symbol.clone())
            .absolute(trade.maker_fee.to_string().parse::<u64>().unwrap_or(0));

        counter!("fees_collected_taker", "symbol" => self.symbol.clone())
            .absolute(trade.taker_fee.to_string().parse::<u64>().unwrap_or(0));
    }

    /// Record match latency
    pub fn record_match_latency(&self, latency_ns: u64) {
        histogram!("match_latency_ns", "symbol" => self.symbol.clone()).record(latency_ns as f64);
    }

    /// Update bid depth gauge
    pub fn update_bid_depth(&self, depth: u64) {
        gauge!("bid_depth", "symbol" => self.symbol.clone()).set(depth as f64);
    }

    /// Update ask depth gauge
    pub fn update_ask_depth(&self, depth: u64) {
        gauge!("ask_depth", "symbol" => self.symbol.clone()).set(depth as f64);
    }

    /// Update best bid gauge
    pub fn update_best_bid(&self, price: &str) {
        gauge!("best_bid", "symbol" => self.symbol.clone())
            .set(price.parse::<f64>().unwrap_or(0.0));
    }

    /// Update best ask gauge
    pub fn update_best_ask(&self, price: &str) {
        gauge!("best_ask", "symbol" => self.symbol.clone())
            .set(price.parse::<f64>().unwrap_or(0.0));
    }

    /// Update spread gauge
    pub fn update_spread(&self, spread: &str) {
        gauge!("spread", "symbol" => self.symbol.clone()).set(spread.parse::<f64>().unwrap_or(0.0));
    }

    /// Record order validation error
    pub fn record_validation_error(&self, error_type: &str) {
        counter!("validation_errors_total", "error_type" => error_type.to_string(), "symbol" => self.symbol.clone())
            .increment(1);
    }

    /// Record self-trade prevention
    pub fn record_self_trade_prevented(&self) {
        counter!("self_trades_prevented_total", "symbol" => self.symbol.clone()).increment(1);
    }
}

/// Event publisher trait for streaming order book events
pub trait EventPublisher: Send + Sync {
    /// Publish an event to subscribers
    fn publish(&self, event: OrderBookEvent);

    /// Publish multiple events in batch
    fn publish_batch(&self, events: Vec<OrderBookEvent>) {
        for event in events {
            self.publish(event);
        }
    }
}

/// In-memory event collector for testing
pub struct InMemoryEventCollector {
    pub events: std::sync::Mutex<Vec<OrderBookEvent>>,
}

impl InMemoryEventCollector {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn get_events(&self) -> Vec<OrderBookEvent> {
        self.events.lock().unwrap().clone()
    }

    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl Default for InMemoryEventCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl EventPublisher for InMemoryEventCollector {
    fn publish(&self, event: OrderBookEvent) {
        self.events.lock().unwrap().push(event);
    }
}

/// Console event publisher for debugging
pub struct ConsoleEventPublisher;

impl Default for ConsoleEventPublisher {
    fn default() -> Self {
        Self
    }
}

impl EventPublisher for ConsoleEventPublisher {
    fn publish(&self, event: OrderBookEvent) {
        println!("[EVENT] {:?}", event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_metrics_creation() {
        let metrics = OrderBookMetrics::new("BTC/USD");
        assert_eq!(metrics.symbol, "BTC/USD");
    }

    #[test]
    fn test_event_collector() {
        let collector = InMemoryEventCollector::new();

        collector.publish(OrderBookEvent::OrderAdded {
            order: crate::orderbook::order::Order::new_limit(
                Side::Buy,
                100,
                dec!(50000.00),
                "BTC/USD".to_string(),
                "account1".to_string(),
            ),
        });

        let events = collector.get_events();
        assert_eq!(events.len(), 1);
    }
}
