# HFT Matching Engine

[![Build Status](https://img.shields.io/github/actions/workflow/status/mansoorali90/matching-engine/ci.yml)](https://github.com/mansoorali90/matching-engine/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://rust-lang.org)

A high-performance, production-ready order matching engine for decentralized exchanges (DeFi) and traditional trading systems, built in Rust.

## 🚀 Features

### Core Functionality

- **Price-Time Priority Matching** - FIFO matching algorithm ensuring fair order execution
- **Multiple Order Types** - Limit, Market, IOC (Immediate-or-Cancel), FOK (Fill-or-Kill)
- **Self-Trade Prevention** - Prevents accounts from trading with themselves
- **Partial Fill Support** - Orders can be partially filled with remaining quantity preserved
- **Order Cancellation** - Full support for order cancellation with proper cleanup

### Production Features

- **Decimal Precision** - Uses `rust_decimal` for accurate price/quantity calculations (no floating-point errors)
- **Circuit Breaker** - Price band protection to prevent extreme price movements
- **Fee System** - Configurable maker/taker fees with basis point precision
- **Event Streaming** - Real-time order book events for external systems
- **Metrics Collection** - Prometheus-compatible metrics for monitoring
- **UUID-based IDs** - Collision-resistant order and trade identifiers

### Safety & Validation

- Order validation (price/quantity limits)
- Duplicate order detection
- Insufficient balance checks (extensible)
- Order expiration support (GTD orders)

## 📊 Performance Benchmarks

| Operation              | Latency     | Throughput       |
| ---------------------- | ----------- | ---------------- |
| Single Trade Execution | **4.5 µs**  | ~220K trades/sec |
| 100 Order Matches      | **707 µs**  | ~7 µs/trade      |
| 1000 Order Matches     | **31.5 ms** | ~31 µs/trade     |
| Order Insertion (1000) | **1.36 ms** | ~735K orders/sec |

_Benchmarks run on Apple M1, release mode optimized_

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      OrderBook                               │
├─────────────────────────────────────────────────────────────┤
│  Bids (BTreeMap)  │  Asks (BTreeMap)  │  All Orders (HashMap)│
│  Price → Orders   │  Price → Orders   │  OrderID → Order    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   MatchingEngine                             │
│  • Price-Time Priority  • Self-Trade Prevention             │
│  • Fee Calculation      • Trade Generation                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Trade Execution                            │
│  • Trade ID Generation  • Fee Distribution                  │
│  • Event Publishing     • Metrics Update                    │
└─────────────────────────────────────────────────────────────┘
```

## 📦 Installation

```bash
# Clone the repository
git clone https://github.com/mansoorali90/matching-engine.git
cd matching-engine

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run benchmarks
cargo bench

# Run the demo
cargo run --release
```

## 🛠️ Usage

### Basic Example

```rust
use hft_my_matching_engine_1::orderbook::{Order, OrderBook, Side, FeeConfig};
use rust_decimal_macros::dec;

fn main() {
    // Create a new order book
    let mut orderbook = OrderBook::new("BTC/USD".to_string());

    // Create orders
    let buy_order = Order::new_limit(
        Side::Buy,
        100,                    // quantity
        dec!(50000.00),         // price
        "BTC/USD".to_string(),
        "trader_1".to_string(),
    );

    let sell_order = Order::new_limit(
        Side::Sell,
        100,
        dec!(50000.00),
        "BTC/USD".to_string(),
        "trader_2".to_string(),
    );

    // Insert orders
    orderbook.insert_order(buy_order).unwrap();
    orderbook.insert_order(sell_order).unwrap();

    // Match orders
    let trades = orderbook.match_orders();

    for trade in &trades {
        println!(
            "Trade: {} {} @ {} (Maker: {})",
            trade.quantity,
            trade.symbol,
            trade.price,
            trade.maker_side
        );
    }
}
```

### Advanced Configuration

```rust
use hft_my_matching_engine_1::orderbook::{OrderBook, OrderBookConfig, FeeConfig};
use rust_decimal_macros::dec;

// Custom configuration
let config = OrderBookConfig {
    max_price: dec!(1_000_000.00),           // $1M max price
    max_quantity: 10_000_000,                 // 10M units max
    fee_config: FeeConfig::new(10, 20),      // 0.10% maker, 0.20% taker
    circuit_breaker_config: Some((500, 60)), // 5% band, 60s cooldown
    enable_self_trade_prevention: true,
};

let orderbook = OrderBook::with_config(
    "BTC/USD".to_string(),
    config,
    Some(Box::new(ConsoleEventPublisher)),
);
```

### Order Types

```rust
// Limit Order (GTC - Good Til Cancelled)
let limit_order = Order::new_limit(
    Side::Buy,
    100,
    dec!(50000.00),
    "BTC/USD".to_string(),
    "trader_1".to_string(),
);

// Market Order (executes immediately at best price)
let market_order = Order::new_market(
    Side::Buy,
    100,
    "BTC/USD".to_string(),
    "trader_1".to_string(),
);
```

## 📈 API Reference

### OrderBook Methods

| Method                                   | Description                           |
| ---------------------------------------- | ------------------------------------- |
| `new(symbol)`                            | Create order book with default config |
| `with_config(symbol, config, publisher)` | Create with custom config             |
| `insert_order(order)`                    | Add order to book                     |
| `cancel_order(order_id)`                 | Cancel existing order                 |
| `match_orders()`                         | Execute order matching                |
| `get_best_bid()`                         | Get highest bid order                 |
| `get_best_ask()`                         | Get lowest ask order                  |
| `get_bid_depth()`                        | Total bid quantity                    |
| `get_ask_depth()`                        | Total ask quantity                    |

### Events

```rust
pub enum OrderBookEvent {
    OrderAdded { order: Order },
    OrderCancelled { order_id: String, reason: String },
    TradeExecuted { trade: Trade },
    PriceLevelChanged { side: Side, price: String, depth: u64 },
    CircuitBreakerTriggered { symbol: String },
    CircuitBreakerReset { symbol: String },
}
```

## 🔧 Configuration Options

```rust
pub struct OrderBookConfig {
    /// Maximum allowed price
    pub max_price: Decimal,

    /// Maximum quantity per order
    pub max_quantity: u64,

    /// Fee configuration (maker/taker in basis points)
    pub fee_config: FeeConfig,

    /// Circuit breaker (price band %, cooldown seconds)
    pub circuit_breaker_config: Option<(u32, u64)>,

    /// Enable self-trade prevention
    pub enable_self_trade_prevention: bool,
}
```

## 🧪 Testing

The project includes comprehensive test coverage:

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_order_matching

# Run with output
cargo test -- --nocapture

# Run benchmarks
cargo bench
```

### Test Coverage

- ✅ Order book creation and management
- ✅ Order insertion (buy/sell)
- ✅ Order matching and execution
- ✅ Partial fills
- ✅ Multiple matches
- ✅ Self-trade prevention
- ✅ Fee calculation
- ✅ Order cancellation
- ✅ Price-time priority
- ✅ Circuit breaker validation

## 📊 Metrics

The engine exports Prometheus-compatible metrics:

| Metric                  | Type      | Description             |
| ----------------------- | --------- | ----------------------- |
| `orders_received_total` | Counter   | Orders received by side |
| `orders_matched_total`  | Counter   | Total orders matched    |
| `volume_matched_total`  | Counter   | Total volume matched    |
| `trades_executed_total` | Counter   | Total trades executed   |
| `trade_volume_total`    | Counter   | Total trade volume      |
| `match_latency_ns`      | Histogram | Matching latency        |
| `bid_depth`             | Gauge     | Current bid depth       |
| `ask_depth`             | Gauge     | Current ask depth       |
| `best_bid`              | Gauge     | Best bid price          |
| `best_ask`              | Gauge     | Best ask price          |
| `spread`                | Gauge     | Current spread          |

## 🔐 Safety Features

1. **No Floating-Point Arithmetic** - All prices use `Decimal` type
2. **Overflow Protection** - Checked arithmetic operations
3. **Order Validation** - Price/quantity limits enforced
4. **Circuit Breakers** - Prevents extreme price movements
5. **Self-Trade Prevention** - Configurable per order book
6. **Unique Identifiers** - UUID-based order and trade IDs

## 🚧 Future Enhancements

- [ ] Async/await support for high concurrency
- [ ] Redis-backed order persistence
- [ ] WebSocket API for real-time updates
- [ ] Order book snapshot/restore
- [ ] Market data feed integration
- [ ] Smart contract integration for DeFi
- [ ] Lock-free data structures for higher throughput
- [ ] GPU-accelerated matching

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📧 Contact

For questions or support, please open an issue on GitHub.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/)
- Uses [rust_decimal](https://crates.io/crates/rust_decimal) for precise arithmetic
- Benchmarking with [criterion](https://crates.io/crates/criterion)
