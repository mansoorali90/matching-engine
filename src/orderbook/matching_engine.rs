use crate::orderbook::order_book::OrderBook;
use crate::orderbook::side::Side;
use crate::orderbook::trade::Trade;

pub struct MatchingEngine;

impl MatchingEngine {
    pub fn match_orders(orderbook: &mut OrderBook) -> Vec<Trade> {
        let mut trades = Vec::new();

        loop {
            // Get the best bid and ask prices (immutable access)
            let best_bid_price = orderbook.get_bids().last_key_value().map(|(p, _)| *p);
            let best_ask_price = orderbook.get_asks().first_key_value().map(|(p, _)| *p);

            match (best_bid_price, best_ask_price) {
                (Some(bid_price), Some(ask_price)) if bid_price >= ask_price => {
                    // Prices match (highest bid >= lowest ask)
                    // Try to match a single pair at this price level
                    let match_result = Self::match_at_price_levels(orderbook, bid_price, ask_price);

                    match match_result {
                        Some(trade) => trades.push(trade),
                        None => break, // No more orders at one of the price levels
                    }
                }
                _ => break, // No more matches possible (bid < ask or no orders)
            }
        }

        trades
    }

    /// Match orders at specific price levels.
    /// Returns Some(Trade) if a match was made, None if either queue is empty.
    fn match_at_price_levels(
        orderbook: &mut OrderBook,
        bid_price: u64,
        ask_price: u64,
    ) -> Option<Trade> {
        // Check if both queues have orders (using immutable access)
        let bid_has_orders = orderbook
            .get_bids()
            .get(&bid_price)
            .map(|q| !q.is_empty())
            .unwrap_or(false);
        let ask_has_orders = orderbook
            .get_asks()
            .get(&ask_price)
            .map(|q| !q.is_empty())
            .unwrap_or(false);

        if !bid_has_orders || !ask_has_orders {
            return None;
        }

        // Get order details for the match (immutable access)
        let match_data = {
            let bid_queue = orderbook.get_bids().get(&bid_price).unwrap();
            let ask_queue = orderbook.get_asks().get(&ask_price).unwrap();
            let bid_order = bid_queue.front().unwrap();
            let ask_order = ask_queue.front().unwrap();

            // Check for self-trade
            if bid_order.account_id == ask_order.account_id {
                // Self-trade detected - cancel the older order (lower sequence number)
                let order_to_cancel = if bid_order.sequence_number < ask_order.sequence_number {
                    bid_order.order_id.to_string()
                } else {
                    ask_order.order_id.to_string()
                };
                let _ = orderbook.cancel_order(&order_to_cancel);
                return None;
            }

            // Calculate matched quantity
            let matched_quantity = std::cmp::min(
                bid_order.remaining_quantity(),
                ask_order.remaining_quantity(),
            );

            if matched_quantity == 0 {
                return None;
            }

            // Determine maker/taker (the order that was placed first is the maker)
            let maker_side = if bid_order.sequence_number < ask_order.sequence_number {
                Side::Buy
            } else {
                Side::Sell
            };

            // Use the maker's price as the trade price (price-time priority)
            let trade_price = if maker_side == Side::Buy {
                bid_order.price
            } else {
                ask_order.price
            };

            // Calculate fees
            let (maker_fee, taker_fee) = orderbook
                .get_fee_config()
                .calculate_fees(trade_price, matched_quantity);

            Some((
                matched_quantity,
                trade_price,
                bid_order.order_id.clone(),
                ask_order.order_id.clone(),
                bid_order.account_id.clone(),
                ask_order.account_id.clone(),
                maker_side,
                maker_fee,
                taker_fee,
            ))
        };

        let (
            matched_quantity,
            trade_price,
            bid_order_id,
            ask_order_id,
            buyer_account_id,
            seller_account_id,
            maker_side,
            maker_fee,
            taker_fee,
        ) = match_data?;

        // Process bid queue (mutable access, separate scope)
        let bid_filled = {
            let bid_queue = orderbook.get_bids_mut().get_mut(&bid_price).unwrap();
            let bid_order = bid_queue.front_mut().unwrap();
            let fill_result = bid_order.fill(matched_quantity);

            match fill_result {
                Ok(()) => bid_order.is_filled(),
                Err(_) => true, // If fill fails, remove the order
            }
        };

        // Process ask queue (mutable access, separate scope)
        let ask_filled = {
            let ask_queue = orderbook.get_asks_mut().get_mut(&ask_price).unwrap();
            let ask_order = ask_queue.front_mut().unwrap();
            let fill_result = ask_order.fill(matched_quantity);

            match fill_result {
                Ok(()) => ask_order.is_filled(),
                Err(_) => true, // If fill fails, remove the order
            }
        };

        // Clean up filled orders and empty price levels
        if bid_filled {
            if let Some(bid_queue) = orderbook.get_bids_mut().get_mut(&bid_price) {
                bid_queue.pop_front();
                if bid_queue.is_empty() {
                    orderbook.get_bids_mut().remove(&bid_price);
                }
            }
            // Cancel the order to clean up all_orders and account_orders
            let _ = orderbook.cancel_order(&bid_order_id.to_string());
        }

        if ask_filled {
            if let Some(ask_queue) = orderbook.get_asks_mut().get_mut(&ask_price) {
                ask_queue.pop_front();
                if ask_queue.is_empty() {
                    orderbook.get_asks_mut().remove(&ask_price);
                }
            }
            // Cancel the order to clean up all_orders and account_orders
            let _ = orderbook.cancel_order(&ask_order_id.to_string());
        }

        // Update circuit breaker reference price
        orderbook.update_reference_price(trade_price);

        // Create and return the trade
        Some(Trade::new(
            trade_price,
            matched_quantity,
            bid_order_id,
            ask_order_id,
            buyer_account_id,
            seller_account_id,
            orderbook.get_symbol().to_string(),
            maker_side,
            maker_fee,
            taker_fee,
        ))
    }
}
