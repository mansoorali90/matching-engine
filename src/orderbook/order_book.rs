// Allow dead_code for public API that may be used by external code
#![allow(dead_code)]

use crate::orderbook::circuit_breaker::CircuitBreaker;
use crate::orderbook::matching_engine::MatchingEngine;
use crate::orderbook::metrics::{EventPublisher, OrderBookEvent, OrderBookMetrics};
use crate::orderbook::order::{Order, OrderError, current_time_ns};
use crate::orderbook::side::Side;
use crate::orderbook::trade::{FeeConfig, Trade};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

/// Configuration for the order book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookConfig {
    /// Maximum price allowed
    pub max_price: Decimal,
    /// Maximum quantity allowed per order
    pub max_quantity: u64,
    /// Fee configuration
    pub fee_config: FeeConfig,
    /// Circuit breaker configuration (price band bps, cooldown seconds)
    pub circuit_breaker_config: Option<(u32, u64)>,
    /// Enable self-trade prevention
    pub enable_self_trade_prevention: bool,
}

impl Default for OrderBookConfig {
    fn default() -> Self {
        Self {
            max_price: Decimal::new(1_000_000_000, 2), // $10M max
            max_quantity: 1_000_000_000,               // 1B units max
            fee_config: FeeConfig::default(),
            circuit_breaker_config: Some((500, 60)), // 5% band, 60s cooldown
            enable_self_trade_prevention: true,
        }
    }
}

/// Order book for a single trading pair
pub struct OrderBook {
    /// Bids sorted by price (highest to lowest via reverse iteration)
    bids: BTreeMap<u64, VecDeque<Order>>,
    /// Asks sorted by price (lowest to highest)
    asks: BTreeMap<u64, VecDeque<Order>>,
    /// Trading pair symbol
    symbol: String,
    /// All active orders by ID
    all_orders: HashMap<String, Order>,
    /// Configuration
    config: OrderBookConfig,
    /// Circuit breaker
    circuit_breaker: Option<CircuitBreaker>,
    /// Metrics collector
    metrics: OrderBookMetrics,
    /// Event publisher
    event_publisher: Option<Box<dyn EventPublisher>>,
    /// Sequence number counter
    sequence_number: u64,
    /// Account IDs with active orders (for self-trade prevention)
    account_orders: HashMap<String, HashSet<String>>,
}

impl OrderBook {
    /// Create a new order book with default configuration
    pub fn new(symbol: String) -> Self {
        Self::with_config(symbol, OrderBookConfig::default(), None)
    }

    /// Create a new order book with custom configuration
    pub fn with_config(
        symbol: String,
        config: OrderBookConfig,
        event_publisher: Option<Box<dyn EventPublisher>>,
    ) -> Self {
        let metrics = OrderBookMetrics::new(&symbol);

        let circuit_breaker = config.circuit_breaker_config.map(|(bps, cooldown)| {
            let mut cb = CircuitBreaker::new(bps, cooldown);
            cb.set_reference_price(config.max_price / Decimal::from(2)); // Default reference
            cb
        });

        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            symbol,
            all_orders: HashMap::new(),
            config,
            circuit_breaker,
            metrics,
            event_publisher,
            sequence_number: 0,
            account_orders: HashMap::new(),
        }
    }

    /// Set the event publisher
    pub fn set_event_publisher(&mut self, publisher: Box<dyn EventPublisher>) {
        self.event_publisher = Some(publisher);
    }

    /// Get the next sequence number
    fn next_sequence(&mut self) -> u64 {
        self.sequence_number += 1;
        self.sequence_number
    }

    /// Publish an event
    fn publish_event(&self, event: OrderBookEvent) {
        if let Some(publisher) = &self.event_publisher {
            publisher.publish(event);
        }
    }

    /// Insert an order into the order book
    pub fn insert_order(&mut self, mut order: Order) -> Result<(), OrderError> {
        // Validate the order
        order.validate(self.config.max_price, self.config.max_quantity)?;

        // Check circuit breaker
        if let Some(cb) = &self.circuit_breaker {
            cb.validate_price(order.price)?;
        }

        // Check for duplicate order ID
        if self.all_orders.contains_key(&order.order_id.to_string()) {
            return Err(OrderError::DuplicateOrderId(order.order_id.to_string()));
        }

        // Set sequence number
        order.sequence_number = self.next_sequence();

        // Store order ID for self-trade prevention
        if self.config.enable_self_trade_prevention {
            self.account_orders
                .entry(order.account_id.clone())
                .or_insert_with(HashSet::new)
                .insert(order.order_id.to_string());
        }

        // Store the order in our global map
        self.all_orders
            .insert(order.order_id.to_string(), order.clone());

        // Insert into the appropriate tree based on side
        match order.side {
            Side::Buy => {
                let price_key = price_to_key(order.price);
                self.bids
                    .entry(price_key)
                    .or_insert_with(VecDeque::new)
                    .push_back(order.clone());
            }
            Side::Sell => {
                let price_key = price_to_key(order.price);
                self.asks
                    .entry(price_key)
                    .or_insert_with(VecDeque::new)
                    .push_back(order.clone());
            }
        }

        // Emit event
        let order_side = order.side;
        self.publish_event(OrderBookEvent::OrderAdded { order });

        // Update metrics
        self.metrics.record_order_received(order_side);
        self.update_depth_metrics();

        Ok(())
    }

    /// Cancel an order by ID
    pub fn cancel_order(&mut self, order_id: &str) -> Result<Order, OrderError> {
        // Remove from global map
        let order = self
            .all_orders
            .remove(order_id)
            .ok_or_else(|| OrderError::OrderNotFound(order_id.to_string()))?;

        // Remove from account tracking
        if self.config.enable_self_trade_prevention {
            if let Some(orders) = self.account_orders.get_mut(&order.account_id) {
                orders.remove(order_id);
            }
        }

        // Remove from the appropriate tree based on side
        match order.side {
            Side::Buy => {
                let price_key = price_to_key(order.price);
                if let Some(order_queue) = self.bids.get_mut(&price_key) {
                    order_queue.retain(|o| o.order_id.to_string() != order_id);
                    // Clean up empty queues
                    if order_queue.is_empty() {
                        self.bids.remove(&price_key);
                    }
                }
            }
            Side::Sell => {
                let price_key = price_to_key(order.price);
                if let Some(order_queue) = self.asks.get_mut(&price_key) {
                    order_queue.retain(|o| o.order_id.to_string() != order_id);
                    // Clean up empty queues
                    if order_queue.is_empty() {
                        self.asks.remove(&price_key);
                    }
                }
            }
        }

        // Emit event
        self.publish_event(OrderBookEvent::OrderCancelled {
            order_id: order_id.to_string(),
            reason: "User requested".to_string(),
        });

        // Update metrics
        self.metrics.record_order_cancelled(order.side);
        self.update_depth_metrics();

        Ok(order)
    }

    /// Match orders and return executed trades
    pub fn match_orders(&mut self) -> Vec<Trade> {
        let start_time = current_time_ns();
        let trades = MatchingEngine::match_orders(self);
        let latency = current_time_ns() - start_time;

        for trade in &trades {
            self.metrics.record_trade(trade);
            self.publish_event(OrderBookEvent::TradeExecuted {
                trade: trade.clone(),
            });
        }

        self.metrics.record_match_latency(latency);
        self.update_depth_metrics();
        self.update_price_metrics();

        trades
    }

    /// Check if two orders would result in a self-trade
    fn would_self_trade(&self, buyer_account_id: &str, seller_account_id: &str) -> bool {
        if !self.config.enable_self_trade_prevention {
            return false;
        }
        buyer_account_id == seller_account_id
    }

    /// Get the best bid order
    pub fn get_best_bid(&self) -> Option<&Order> {
        self.bids
            .values()
            .flat_map(|queue| queue.front())
            .max_by_key(|order| order.price)
    }

    /// Get the best ask order
    pub fn get_best_ask(&self) -> Option<&Order> {
        self.asks
            .values()
            .flat_map(|queue| queue.front())
            .min_by_key(|order| order.price)
    }

    /// Get an order by ID
    pub fn get_order(&self, order_id: &str) -> Option<&Order> {
        self.all_orders.get(order_id)
    }

    /// Get mutable reference to an order by ID
    pub fn get_order_mut(&mut self, order_id: &str) -> Option<&mut Order> {
        self.all_orders.get_mut(order_id)
    }

    /// Get all bids (read-only)
    pub fn get_bids(&self) -> &BTreeMap<u64, VecDeque<Order>> {
        &self.bids
    }

    /// Get all asks (read-only)
    pub fn get_asks(&self) -> &BTreeMap<u64, VecDeque<Order>> {
        &self.asks
    }

    /// Get mutable bids
    pub fn get_bids_mut(&mut self) -> &mut BTreeMap<u64, VecDeque<Order>> {
        &mut self.bids
    }

    /// Get mutable asks
    pub fn get_asks_mut(&mut self) -> &mut BTreeMap<u64, VecDeque<Order>> {
        &mut self.asks
    }

    /// Get the trading symbol
    pub fn get_symbol(&self) -> &str {
        &self.symbol
    }

    /// Get the fee configuration
    pub fn get_fee_config(&self) -> &FeeConfig {
        &self.config.fee_config
    }

    /// Get all active orders for an account
    pub fn get_account_orders(&self, account_id: &str) -> Vec<&Order> {
        self.account_orders
            .get(account_id)
            .map(|order_ids| {
                order_ids
                    .iter()
                    .filter_map(|id| self.all_orders.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total bid depth (quantity)
    pub fn get_bid_depth(&self) -> u64 {
        self.bids
            .values()
            .flat_map(|q| q.iter())
            .map(|o| o.quantity)
            .sum()
    }

    /// Get total ask depth (quantity)
    pub fn get_ask_depth(&self) -> u64 {
        self.asks
            .values()
            .flat_map(|q| q.iter())
            .map(|o| o.quantity)
            .sum()
    }

    /// Update depth metrics
    fn update_depth_metrics(&self) {
        self.metrics.update_bid_depth(self.get_bid_depth());
        self.metrics.update_ask_depth(self.get_ask_depth());
    }

    /// Update price metrics
    fn update_price_metrics(&self) {
        if let Some(best_bid) = self.get_best_bid() {
            self.metrics.update_best_bid(&best_bid.price.to_string());
        }
        if let Some(best_ask) = self.get_best_ask() {
            self.metrics.update_best_ask(&best_ask.price.to_string());
        }

        if let (Some(best_bid), Some(best_ask)) = (self.get_best_bid(), self.get_best_ask()) {
            let spread = best_ask.price - best_bid.price;
            self.metrics.update_spread(&spread.to_string());
        }
    }

    /// Update the reference price for the circuit breaker
    pub fn update_reference_price(&mut self, price: Decimal) {
        if let Some(cb) = &mut self.circuit_breaker {
            cb.update_reference_price(price);
        }
    }

    /// Manually trigger the circuit breaker
    pub fn trigger_circuit_breaker(&mut self) {
        if let Some(cb) = &mut self.circuit_breaker {
            cb.trigger();
            let reference_price = cb
                .reference_price
                .map(|p| p.to_string())
                .unwrap_or_default();
            let symbol = self.symbol.clone();
            self.publish_event(OrderBookEvent::CircuitBreakerTriggered {
                symbol,
                reference_price,
            });
        }
    }

    /// Reset the circuit breaker
    pub fn reset_circuit_breaker(&mut self) {
        if let Some(cb) = &mut self.circuit_breaker {
            cb.reset();
            self.publish_event(OrderBookEvent::CircuitBreakerReset {
                symbol: self.symbol.clone(),
            });
        }
    }

    /// Print the current order book state
    pub fn print_status(&self) {
        println!("=== OrderBook Status for {} ===", self.symbol);
        println!("Bids (highest to lowest price):");
        for (price, orders) in self.bids.iter().rev() {
            println!(
                "  Price {}: {} orders, total qty: {}",
                price,
                orders.len(),
                orders.iter().map(|o| o.quantity).sum::<u64>()
            );
        }
        println!("Asks (lowest to highest price):");
        for (price, orders) in self.asks.iter() {
            println!(
                "  Price {}: {} orders, total qty: {}",
                price,
                orders.len(),
                orders.iter().map(|o| o.quantity).sum::<u64>()
            );
        }
        println!(
            "Total bids: {}, total qty: {}",
            self.bids.len(),
            self.get_bid_depth()
        );
        println!(
            "Total asks: {}, total qty: {}",
            self.asks.len(),
            self.get_ask_depth()
        );
        if let Some(best_bid) = self.get_best_bid() {
            println!("Best bid: {} @ {}", best_bid.quantity, best_bid.price);
        }
        if let Some(best_ask) = self.get_best_ask() {
            println!("Best ask: {} @ {}", best_ask.quantity, best_ask.price);
        }
        println!("==============================");
    }
}

/// Convert a Decimal price to a u64 key for BTreeMap ordering
/// Uses the underlying representation for consistent ordering
fn price_to_key(price: Decimal) -> u64 {
    // Scale price to integer representation
    let scaled = price * Decimal::from(100_000_000); // 8 decimal places
    scaled.to_string().parse().unwrap_or(0)
}
