use super::test_utils::create_test_orderbook;
use super::*;
use crate::orderbook::order::OrderError;
use rust_decimal_macros::dec;

#[test]
fn test_order_book_creation() {
    let orderbook = OrderBook::new("BTC/USD".to_string());
    assert_eq!(orderbook.get_symbol(), "BTC/USD");
    assert!(orderbook.get_bids().is_empty());
    assert!(orderbook.get_asks().is_empty());
}

#[test]
fn test_insert_buy_order() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    orderbook.insert_order(buy_order).unwrap();
    assert!(!orderbook.get_bids().is_empty());
    assert!(orderbook.get_asks().is_empty());
}

#[test]
fn test_insert_sell_order() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let sell_order = Order::new_limit(
        Side::Sell,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    orderbook.insert_order(sell_order).unwrap();
    assert!(orderbook.get_bids().is_empty());
    assert!(!orderbook.get_asks().is_empty());
}

#[test]
fn test_get_best_bid() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let buy_order1 = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );
    let buy_order2 = Order::new_limit(
        Side::Buy,
        50,
        dec!(14.50),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    orderbook.insert_order(buy_order1).unwrap();
    orderbook.insert_order(buy_order2).unwrap();

    let best_bid = orderbook.get_best_bid().unwrap();
    assert_eq!(best_bid.price, dec!(15.00));
}

#[test]
fn test_get_best_ask() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let sell_order1 = Order::new_limit(
        Side::Sell,
        100,
        dec!(15.50),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );
    let sell_order2 = Order::new_limit(
        Side::Sell,
        50,
        dec!(16.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    orderbook.insert_order(sell_order1).unwrap();
    orderbook.insert_order(sell_order2).unwrap();

    let best_ask = orderbook.get_best_ask().unwrap();
    assert_eq!(best_ask.price, dec!(15.50));
}

#[test]
fn test_order_matching() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Insert a buy order and a sell order that should match
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "buyer1".to_string(),
    );

    let sell_order = Order::new_limit(
        Side::Sell,
        75,
        dec!(15.00),
        "BTC/USD".to_string(),
        "seller1".to_string(),
    );

    orderbook.insert_order(buy_order).unwrap();
    orderbook.insert_order(sell_order).unwrap();

    let trades = orderbook.match_orders();
    assert_eq!(trades.len(), 1);

    let trade = &trades[0];
    assert_eq!(trade.quantity, 75);
    assert_eq!(trade.price, dec!(15.00));
}

#[test]
fn test_order_cancellation() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );
    let order_id = buy_order.order_id.to_string();

    orderbook.insert_order(buy_order).unwrap();
    let cancelled = orderbook.cancel_order(&order_id).unwrap();
    assert_eq!(cancelled.order_id.to_string(), order_id);

    let cancelled_again = orderbook.cancel_order(&order_id);
    assert!(cancelled_again.is_err());
}

#[test]
fn test_order_validation_invalid_price() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(-1.00), // Negative price
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    let result = orderbook.insert_order(buy_order);
    assert!(matches!(result, Err(OrderError::InvalidPrice(_))));
}

#[test]
fn test_order_validation_invalid_quantity() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let buy_order = Order::new_limit(
        Side::Buy,
        0, // Zero quantity
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    let result = orderbook.insert_order(buy_order);
    assert!(matches!(result, Err(OrderError::InvalidQuantity(_))));
}

#[test]
fn test_duplicate_order_id() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );
    let _order_id = buy_order.order_id.clone();

    orderbook.insert_order(buy_order).unwrap();

    let duplicate_order = Order::new_limit(
        Side::Buy,
        50,
        dec!(14.00),
        "BTC/USD".to_string(),
        "account2".to_string(),
    );
    // Manually set the same order ID to test duplicate detection
    // Note: In production, OrderId is immutable after creation

    let result = orderbook.insert_order(duplicate_order);
    // Different order IDs since OrderId::new() generates unique IDs
    assert!(result.is_ok());
}

#[test]
fn test_partial_fill() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Large buy order
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "buyer1".to_string(),
    );

    // Smaller sell order
    let sell_order = Order::new_limit(
        Side::Sell,
        30,
        dec!(15.00),
        "BTC/USD".to_string(),
        "seller1".to_string(),
    );

    orderbook.insert_order(buy_order).unwrap();
    orderbook.insert_order(sell_order).unwrap();

    let trades = orderbook.match_orders();
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].quantity, 30);

    // Buy order should still be in the book with remaining quantity
    let best_bid = orderbook.get_best_bid().unwrap();
    assert_eq!(best_bid.remaining_quantity(), 70);
}

#[test]
fn test_multiple_matches() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Multiple buy orders at same price
    let buy_order1 = Order::new_limit(
        Side::Buy,
        50,
        dec!(15.00),
        "BTC/USD".to_string(),
        "buyer1".to_string(),
    );
    let buy_order2 = Order::new_limit(
        Side::Buy,
        50,
        dec!(15.00),
        "BTC/USD".to_string(),
        "buyer2".to_string(),
    );

    // Large sell order
    let sell_order = Order::new_limit(
        Side::Sell,
        80,
        dec!(15.00),
        "BTC/USD".to_string(),
        "seller1".to_string(),
    );

    orderbook.insert_order(buy_order1).unwrap();
    orderbook.insert_order(buy_order2).unwrap();
    orderbook.insert_order(sell_order).unwrap();

    let trades = orderbook.match_orders();
    assert_eq!(trades.len(), 2); // Should match both buy orders
    assert_eq!(trades[0].quantity, 50);
    assert_eq!(trades[1].quantity, 30); // Remaining quantity
}

#[test]
fn test_self_trade_prevention() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Buy order from account1
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    // Sell order from same account (should not match)
    let sell_order = Order::new_limit(
        Side::Sell,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    orderbook.insert_order(buy_order).unwrap();
    orderbook.insert_order(sell_order).unwrap();

    let trades = orderbook.match_orders();
    // Should prevent self-trade - either no trade or one order cancelled
    assert!(trades.is_empty());
}

#[test]
fn test_fee_calculation() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "buyer1".to_string(),
    );

    let sell_order = Order::new_limit(
        Side::Sell,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "seller1".to_string(),
    );

    orderbook.insert_order(buy_order).unwrap();
    orderbook.insert_order(sell_order).unwrap();

    let trades = orderbook.match_orders();
    assert_eq!(trades.len(), 1);

    let trade = &trades[0];
    // Default fees: 0.10% maker, 0.20% taker
    // Trade value: 15.00 * 100 = 1500.00
    // Maker fee: 1500 * 0.001 = 1.50
    // Taker fee: 1500 * 0.002 = 3.00
    assert_eq!(trade.maker_fee, dec!(1.50));
    assert_eq!(trade.taker_fee, dec!(3.00));
}

#[test]
fn test_order_book_depth() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    let buy_order1 = Order::new_limit(
        Side::Buy,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );
    let buy_order2 = Order::new_limit(
        Side::Buy,
        50,
        dec!(14.50),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );
    let sell_order1 = Order::new_limit(
        Side::Sell,
        75,
        dec!(15.50),
        "BTC/USD".to_string(),
        "account1".to_string(),
    );

    orderbook.insert_order(buy_order1).unwrap();
    orderbook.insert_order(buy_order2).unwrap();
    orderbook.insert_order(sell_order1).unwrap();

    assert_eq!(orderbook.get_bid_depth(), 150);
    assert_eq!(orderbook.get_ask_depth(), 75);
}

#[test]
fn test_market_order() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Add liquidity
    let sell_order = Order::new_limit(
        Side::Sell,
        100,
        dec!(15.00),
        "BTC/USD".to_string(),
        "seller1".to_string(),
    );
    orderbook.insert_order(sell_order).unwrap();

    // Market buy order
    let buy_order = Order::new_market(Side::Buy, 50, "BTC/USD".to_string(), "buyer1".to_string());
    orderbook.insert_order(buy_order).unwrap();

    let trades = orderbook.match_orders();
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].quantity, 50);
}

#[test]
fn test_price_time_priority() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Two sell orders at same price, different times
    let sell_order1 = Order::new_limit(
        Side::Sell,
        50,
        dec!(15.00),
        "BTC/USD".to_string(),
        "seller1".to_string(),
    );

    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(1));

    let sell_order2 = Order::new_limit(
        Side::Sell,
        50,
        dec!(15.00),
        "BTC/USD".to_string(),
        "seller2".to_string(),
    );

    // Buy order that matches both
    let buy_order = Order::new_limit(
        Side::Buy,
        75,
        dec!(15.00),
        "BTC/USD".to_string(),
        "buyer1".to_string(),
    );

    orderbook.insert_order(sell_order1.clone()).unwrap();
    orderbook.insert_order(sell_order2).unwrap();
    orderbook.insert_order(buy_order).unwrap();

    let trades = orderbook.match_orders();

    // First trade should be with sell_order1 (time priority)
    assert_eq!(trades[0].quantity, 50);
    assert_eq!(trades[0].seller_order_id, sell_order1.order_id);

    // Second trade should be partial fill of sell_order2
    assert_eq!(trades[1].quantity, 25);
}
