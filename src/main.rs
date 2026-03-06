mod orderbook;

use orderbook::{ConsoleEventPublisher, Order, OrderBook, OrderBookConfig, Side};
use rust_decimal_macros::dec;

fn main() {
    // Create a custom configuration
    let config = OrderBookConfig {
        max_price: dec!(1_000_000.00),
        max_quantity: 10_000_000,
        fee_config: orderbook::FeeConfig::new(10, 20), // 0.10% maker, 0.20% taker
        circuit_breaker_config: Some((500, 60)),       // 5% price band, 60s cooldown
        enable_self_trade_prevention: true,
    };

    // Create a new order book for BTC/USD with event publishing
    let event_publisher = Box::new(ConsoleEventPublisher);
    let mut orderbook =
        OrderBook::with_config("BTC/USD".to_string(), config, Some(event_publisher));

    // Set initial reference price for circuit breaker (mid-price of expected range)
    orderbook.update_reference_price(dec!(50_000.00));

    println!("=== Production-Ready Matching Engine Demo ===\n");

    // Create sample orders with proper API
    let buy_order1 = Order::new_limit(
        Side::Buy,
        100,
        dec!(50_000.00),
        "BTC/USD".to_string(),
        "buyer_account_1".to_string(),
    );
    let buy_order1_id = buy_order1.order_id.to_string();

    let buy_order2 = Order::new_limit(
        Side::Buy,
        50,
        dec!(49_500.00),
        "BTC/USD".to_string(),
        "buyer_account_2".to_string(),
    );

    let sell_order1 = Order::new_limit(
        Side::Sell,
        75,
        dec!(50_500.00),
        "BTC/USD".to_string(),
        "seller_account_1".to_string(),
    );

    let sell_order2 = Order::new_limit(
        Side::Sell,
        25,
        dec!(51_000.00),
        "BTC/USD".to_string(),
        "seller_account_2".to_string(),
    );

    // Insert orders into the order book
    println!("Inserting orders into order book...");
    orderbook
        .insert_order(buy_order1)
        .expect("Failed to insert buy_order1");
    orderbook
        .insert_order(buy_order2)
        .expect("Failed to insert buy_order2");
    orderbook
        .insert_order(sell_order1)
        .expect("Failed to insert sell_order1");
    orderbook
        .insert_order(sell_order2)
        .expect("Failed to insert sell_order2");

    // Display the current state
    println!("\nOrder book state after inserting orders:");
    println!(
        "Best bid: {:?}",
        orderbook
            .get_best_bid()
            .map(|o| format!("{} @ {}", o.quantity, o.price))
    );
    println!(
        "Best ask: {:?}",
        orderbook
            .get_best_ask()
            .map(|o| format!("{} @ {}", o.quantity, o.price))
    );
    println!("Bid depth: {}", orderbook.get_bid_depth());
    println!("Ask depth: {}", orderbook.get_ask_depth());

    // Try to match orders (no match yet since bid < ask)
    println!("\nAttempting to match orders...");
    let matches = orderbook.match_orders();
    if matches.is_empty() {
        println!("No matches found (spread exists)");
    } else {
        for trade in &matches {
            println!(
                "Trade: {} {} @ {} (Maker: {}, Fees: maker={}, taker={})",
                trade.quantity,
                trade.symbol,
                trade.price,
                trade.maker_side,
                trade.maker_fee,
                trade.taker_fee
            );
        }
    }

    // Add a matching sell order
    println!("\n--- Adding matching sell order ---");
    let matching_sell = Order::new_limit(
        Side::Sell,
        50,
        dec!(50_000.00), // Same price as best bid
        "BTC/USD".to_string(),
        "seller_account_3".to_string(),
    );
    orderbook
        .insert_order(matching_sell)
        .expect("Failed to insert matching sell");

    // Match orders
    println!("\nMatching orders...");
    let trades = orderbook.match_orders();
    println!("Number of trades executed: {}", trades.len());
    for trade in &trades {
        println!(
            "  Trade #{}: {} {} @ {} | Buyer: {} | Seller: {} | Maker: {} | Fees: maker={}, taker={}",
            trade.trade_id.to_string()[..8].to_string(),
            trade.quantity,
            trade.symbol,
            trade.price,
            trade.buyer_account_id,
            trade.seller_account_id,
            trade.maker_side,
            trade.maker_fee,
            trade.taker_fee
        );
    }

    // Display state after matching
    println!("\nOrder book state after matching:");
    println!(
        "Best bid: {:?}",
        orderbook
            .get_best_bid()
            .map(|o| format!("{} @ {}", o.remaining_quantity(), o.price))
    );
    println!(
        "Best ask: {:?}",
        orderbook
            .get_best_ask()
            .map(|o| format!("{} @ {}", o.quantity, o.price))
    );

    // Cancel an order
    println!("\n--- Canceling order ---");
    println!("Canceling order '{}'...", buy_order1_id);
    match orderbook.cancel_order(&buy_order1_id) {
        Ok(order) => println!(
            "Cancelled: {} {} @ {}",
            order.quantity, order.symbol, order.price
        ),
        Err(e) => println!("Error: {:?}", e),
    }

    // Display state after cancellation
    println!("\nOrder book state after cancellation:");
    orderbook.print_status();

    // Demonstrate self-trade prevention
    println!("\n--- Testing Self-Trade Prevention ---");
    let self_trade_buy = Order::new_limit(
        Side::Buy,
        30,
        dec!(49_500.00),
        "BTC/USD".to_string(),
        "same_account".to_string(),
    );
    let self_trade_sell = Order::new_limit(
        Side::Sell,
        30,
        dec!(49_500.00),
        "BTC/USD".to_string(),
        "same_account".to_string(),
    );

    orderbook
        .insert_order(self_trade_buy)
        .expect("Failed to insert self-trade buy");
    orderbook
        .insert_order(self_trade_sell)
        .expect("Failed to insert self-trade sell");

    println!("Inserted buy and sell from same account...");
    let self_trades = orderbook.match_orders();
    println!(
        "Self-trades prevented: {} trades executed (should be 0)",
        self_trades.len()
    );

    // Final status
    println!("\n=== Final Order Book Status ===");
    orderbook.print_status();

    println!("\n=== Demo Complete ===");
}
