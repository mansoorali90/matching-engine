use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn create_test_orderbook() -> hft_my_matching_engine_1::orderbook::OrderBook {
    let config = hft_my_matching_engine_1::orderbook::OrderBookConfig {
        max_price: dec!(1_000_000.00),
        max_quantity: 10_000_000,
        fee_config: hft_my_matching_engine_1::orderbook::FeeConfig::new(10, 20),
        circuit_breaker_config: None,
        enable_self_trade_prevention: false,
    };
    hft_my_matching_engine_1::orderbook::OrderBook::with_config("BTC/USD".to_string(), config, None)
}

fn benchmark_order_matching_100_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_100_transactions");

    group.bench_function("match_100_trades", |b| {
        b.iter(|| {
            let mut orderbook = create_test_orderbook();

            for i in 0..100 {
                let buy_order = hft_my_matching_engine_1::orderbook::Order::new_limit(
                    hft_my_matching_engine_1::orderbook::Side::Buy,
                    10,
                    dec!(50.00),
                    "BTC/USD".to_string(),
                    format!("buyer{}", i),
                );
                let sell_order = hft_my_matching_engine_1::orderbook::Order::new_limit(
                    hft_my_matching_engine_1::orderbook::Side::Sell,
                    10,
                    dec!(50.00),
                    "BTC/USD".to_string(),
                    format!("seller{}", i),
                );

                orderbook.insert_order(buy_order).unwrap();
                orderbook.insert_order(sell_order).unwrap();
            }

            let trades = black_box(orderbook.match_orders());
            assert_eq!(trades.len(), 100);
        })
    });

    group.finish();
}

fn benchmark_order_matching_1000_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_1000_transactions");

    group.bench_function("match_1000_trades", |b| {
        b.iter(|| {
            let mut orderbook = create_test_orderbook();

            for i in 0..1000 {
                let buy_order = hft_my_matching_engine_1::orderbook::Order::new_limit(
                    hft_my_matching_engine_1::orderbook::Side::Buy,
                    10,
                    dec!(50.00),
                    "BTC/USD".to_string(),
                    format!("buyer{}", i),
                );
                let sell_order = hft_my_matching_engine_1::orderbook::Order::new_limit(
                    hft_my_matching_engine_1::orderbook::Side::Sell,
                    10,
                    dec!(50.00),
                    "BTC/USD".to_string(),
                    format!("seller{}", i),
                );

                orderbook.insert_order(buy_order).unwrap();
                orderbook.insert_order(sell_order).unwrap();
            }

            let trades = black_box(orderbook.match_orders());
            assert_eq!(trades.len(), 1000);
        })
    });

    group.finish();
}

fn benchmark_order_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("order_insertion");

    for &num_orders in &[10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_orders),
            &num_orders,
            |b, &n| {
                b.iter(|| {
                    let mut orderbook = create_test_orderbook();
                    for i in 0..n {
                        let order = hft_my_matching_engine_1::orderbook::Order::new_limit(
                            hft_my_matching_engine_1::orderbook::Side::Buy,
                            10,
                            dec!(50.00) - Decimal::from(i) * dec!(0.01),
                            "BTC/USD".to_string(),
                            format!("buyer{}", i),
                        );
                        black_box(orderbook.insert_order(order).unwrap());
                    }
                })
            },
        );
    }
    group.finish();
}

fn benchmark_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency_analysis");

    group.bench_function("single_trade_latency", |b| {
        b.iter(|| {
            let mut orderbook = create_test_orderbook();

            let buy_order = hft_my_matching_engine_1::orderbook::Order::new_limit(
                hft_my_matching_engine_1::orderbook::Side::Buy,
                10,
                dec!(50.00),
                "BTC/USD".to_string(),
                "buyer1".to_string(),
            );
            let sell_order = hft_my_matching_engine_1::orderbook::Order::new_limit(
                hft_my_matching_engine_1::orderbook::Side::Sell,
                10,
                dec!(50.00),
                "BTC/USD".to_string(),
                "seller1".to_string(),
            );

            orderbook.insert_order(buy_order).unwrap();
            orderbook.insert_order(sell_order).unwrap();

            let trades = black_box(orderbook.match_orders());
            assert_eq!(trades.len(), 1);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_order_insertion,
    benchmark_order_matching_100_transactions,
    benchmark_order_matching_1000_transactions,
    benchmark_latency,
);

criterion_main!(benches);
