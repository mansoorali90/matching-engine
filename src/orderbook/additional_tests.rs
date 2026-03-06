use super::test_utils::create_test_orderbook;
use super::*;
use crate::orderbook::order::OrderError;
use rust_decimal_macros::dec;

// ============== Additional Comprehensive Tests ==============

#[test]
fn test_empty_orderbook_matching() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let trades = orderbook.match_orders();
    assert!(trades.is_empty());
}

#[test]
fn test_no_crossing_orders() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Buy at 45, Sell at 55 - no match possible
    let buy_order = Order::new_limit(
        Side::Buy,
        100,
        dec!(45.00),
        "BTC/USD".into(),
        "buyer1".into(),
    );
    let sell_order = Order::new_limit(
        Side::Sell,
        100,
        dec!(55.00),
        "BTC/USD".into(),
        "seller1".into(),
    );

    orderbook.insert_order(buy_order).unwrap();
    orderbook.insert_order(sell_order).unwrap();

    let trades = orderbook.match_orders();
    assert!(trades.is_empty());
    assert!(orderbook.get_best_bid().is_some());
    assert!(orderbook.get_best_ask().is_some());
}

#[test]
fn test_large_order_multiple_matches() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Add sell orders at price 45
    for i in 0..10 {
        orderbook
            .insert_order(Order::new_limit(
                Side::Sell,
                10,
                dec!(45.00),
                "BTC/USD".into(),
                format!("seller{}", i).into(),
            ))
            .unwrap();
    }

    // Large buy order at higher price 46
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            80,
            dec!(46.00),
            "BTC/USD".into(),
            "bigbuyer".into(),
        ))
        .unwrap();

    let trades = orderbook.match_orders();
    // Should match 8 sell orders (80 / 10 = 8)
    assert_eq!(trades.len(), 8);
    // 2 sell orders should remain
    assert_eq!(orderbook.get_asks().len(), 2);
}

#[test]
fn test_order_book_price_levels() {
    let mut orderbook = create_test_orderbook("BTC/USD");

    // Add buy orders at different price levels (40, 41, 42)
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            10,
            dec!(40.00),
            "BTC/USD".into(),
            "b1".into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            20,
            dec!(41.00),
            "BTC/USD".into(),
            "b2".into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            30,
            dec!(42.00),
            "BTC/USD".into(),
            "b3".into(),
        ))
        .unwrap();

    // Add sell orders at higher prices (50, 51, 52)
    orderbook
        .insert_order(Order::new_limit(
            Side::Sell,
            15,
            dec!(50.00),
            "BTC/USD".into(),
            "s1".into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Sell,
            25,
            dec!(51.00),
            "BTC/USD".into(),
            "s2".into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Sell,
            35,
            dec!(52.00),
            "BTC/USD".into(),
            "s3".into(),
        ))
        .unwrap();

    assert_eq!(orderbook.get_bids().len(), 3);
    assert_eq!(orderbook.get_asks().len(), 3);
}

#[test]
fn test_cancel_nonexistent_order() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let result = orderbook.cancel_order("nonexistent-id");
    assert!(matches!(result, Err(OrderError::OrderNotFound(_))));
}

#[test]
fn test_fee_config_custom_values() {
    let config = OrderBookConfig {
        max_price: dec!(1_000_000.00),
        max_quantity: 10_000_000,
        fee_config: FeeConfig::new(5, 15),
        circuit_breaker_config: None,
        enable_self_trade_prevention: true,
    };
    let (maker_fee, taker_fee) = config.fee_config.calculate_fees(dec!(100.00), 1000);
    assert_eq!(maker_fee, dec!(50.00));
    assert_eq!(taker_fee, dec!(150.00));
}

#[test]
fn test_zero_quantity_order_rejected() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let order = Order::new_limit(Side::Buy, 0, dec!(50.00), "BTC/USD".into(), "buyer1".into());
    assert!(matches!(
        orderbook.insert_order(order),
        Err(OrderError::InvalidQuantity(_))
    ));
}

#[test]
fn test_order_filled_status() {
    let mut order = Order::new_limit(
        Side::Buy,
        100,
        dec!(50.00),
        "BTC/USD".into(),
        "buyer1".into(),
    );
    assert!(!order.is_filled());
    order.fill(30).unwrap();
    assert!(!order.is_filled());
    order.fill(70).unwrap();
    assert!(order.is_filled());
}

#[test]
fn test_spread_calculation() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            100,
            dec!(45.00),
            "BTC/USD".into(),
            "b1".into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Sell,
            100,
            dec!(55.00),
            "BTC/USD".into(),
            "s1".into(),
        ))
        .unwrap();
    let spread = orderbook.get_best_ask().unwrap().price - orderbook.get_best_bid().unwrap().price;
    assert_eq!(spread, dec!(10.00));
}

#[test]
fn test_account_orders_tracking() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    let account = "trader1";
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            10,
            dec!(45.00),
            "BTC/USD".into(),
            account.into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Sell,
            15,
            dec!(55.00),
            "BTC/USD".into(),
            account.into(),
        ))
        .unwrap();
    assert_eq!(orderbook.get_account_orders(account).len(), 2);
}

#[test]
fn test_trade_id_uniqueness() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    // Add non-matching orders
    for i in 0..5 {
        orderbook
            .insert_order(Order::new_limit(
                Side::Buy,
                10,
                dec!(40.00),
                "BTC/USD".into(),
                format!("b{}", i).into(),
            ))
            .unwrap();
        orderbook
            .insert_order(Order::new_limit(
                Side::Sell,
                10,
                dec!(60.00),
                "BTC/USD".into(),
                format!("s{}", i).into(),
            ))
            .unwrap();
    }
    // No trades since prices don't match
    assert_eq!(orderbook.get_bids().len(), 5);
    assert_eq!(orderbook.get_asks().len(), 5);
}

#[test]
fn test_market_depth_imbalance() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    for i in 0..10 {
        orderbook
            .insert_order(Order::new_limit(
                Side::Buy,
                100,
                dec!(45.00),
                "BTC/USD".into(),
                format!("b{}", i).into(),
            ))
            .unwrap();
    }
    for i in 0..3 {
        orderbook
            .insert_order(Order::new_limit(
                Side::Sell,
                50,
                dec!(55.00),
                "BTC/USD".into(),
                format!("s{}", i).into(),
            ))
            .unwrap();
    }
    assert_eq!(orderbook.get_bid_depth(), 1000);
    assert_eq!(orderbook.get_ask_depth(), 150);
}

#[test]
fn test_trade_value_calculation() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            100,
            dec!(50.00),
            "BTC/USD".into(),
            "b1".into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Sell,
            100,
            dec!(50.00),
            "BTC/USD".into(),
            "s1".into(),
        ))
        .unwrap();
    let trades = orderbook.match_orders();
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].trade_value, dec!(5000.00));
}

#[test]
fn test_exact_match_no_remaining() {
    let mut orderbook = create_test_orderbook("BTC/USD");
    orderbook
        .insert_order(Order::new_limit(
            Side::Buy,
            50,
            dec!(50.00),
            "BTC/USD".into(),
            "b1".into(),
        ))
        .unwrap();
    orderbook
        .insert_order(Order::new_limit(
            Side::Sell,
            50,
            dec!(50.00),
            "BTC/USD".into(),
            "s1".into(),
        ))
        .unwrap();
    let trades = orderbook.match_orders();
    assert_eq!(trades.len(), 1);
    assert!(orderbook.get_best_bid().is_none());
    assert!(orderbook.get_best_ask().is_none());
}
