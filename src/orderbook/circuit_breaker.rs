// Allow dead_code for public API that may be used by external code
#![allow(dead_code)]

use crate::orderbook::order::current_time_ns;
use serde::{Deserialize, Serialize};

/// Circuit breaker for price band protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreaker {
    /// Price band percentage in basis points (e.g., 500 = 5%)
    pub price_band_bps: u32,
    /// Reference price for calculating bands
    pub reference_price: Option<rust_decimal::Decimal>,
    /// Upper price limit
    pub upper_limit: Option<rust_decimal::Decimal>,
    /// Lower price limit
    pub lower_limit: Option<rust_decimal::Decimal>,
    /// Whether the circuit breaker is currently triggered
    pub is_triggered: bool,
    /// Trigger timestamp in nanoseconds
    pub trigger_timestamp_ns: Option<u64>,
    /// Cooldown period in nanoseconds
    pub cooldown_ns: u64,
}

impl CircuitBreaker {
    pub fn new(price_band_bps: u32, cooldown_seconds: u64) -> Self {
        Self {
            price_band_bps,
            reference_price: None,
            upper_limit: None,
            lower_limit: None,
            is_triggered: false,
            trigger_timestamp_ns: None,
            cooldown_ns: cooldown_seconds * 1_000_000_000,
        }
    }

    /// Set the reference price and calculate limits
    pub fn set_reference_price(&mut self, price: rust_decimal::Decimal) {
        self.reference_price = Some(price);

        let band_multiplier =
            rust_decimal::Decimal::from(self.price_band_bps) / rust_decimal::Decimal::from(10_000);

        let band_amount = price * band_multiplier;
        self.upper_limit = Some(price + band_amount);
        self.lower_limit = Some(price - band_amount);
    }

    /// Check if a price is within acceptable bands
    pub fn is_within_bands(&self, price: rust_decimal::Decimal) -> bool {
        if self.is_triggered {
            return false;
        }

        match (self.lower_limit, self.upper_limit) {
            (Some(lower), Some(upper)) => price >= lower && price <= upper,
            _ => true, // No limits set, allow all
        }
    }

    /// Validate a price against the circuit breaker
    pub fn validate_price(
        &self,
        price: rust_decimal::Decimal,
    ) -> Result<(), crate::orderbook::order::OrderError> {
        if !self.is_within_bands(price) {
            return Err(crate::orderbook::order::OrderError::PriceBandViolation {
                price: price.to_string().parse().unwrap_or(0),
                limit: self
                    .upper_limit
                    .or(self.lower_limit)
                    .unwrap_or(rust_decimal::Decimal::ZERO)
                    .to_string()
                    .parse()
                    .unwrap_or(0),
            });
        }
        Ok(())
    }

    /// Trigger the circuit breaker
    pub fn trigger(&mut self) {
        self.is_triggered = true;
        self.trigger_timestamp_ns = Some(current_time_ns());
    }

    /// Reset the circuit breaker
    pub fn reset(&mut self) {
        self.is_triggered = false;
        self.trigger_timestamp_ns = None;
    }

    /// Check if the circuit breaker has cooled down and can be reset
    pub fn try_auto_reset(&mut self) -> bool {
        if !self.is_triggered {
            return true;
        }

        if let Some(trigger_time) = self.trigger_timestamp_ns
            && current_time_ns() - trigger_time >= self.cooldown_ns
        {
            self.reset();
            return true;
        }
        false
    }

    /// Update reference price (e.g., from oracle or last trade)
    pub fn update_reference_price(&mut self, price: rust_decimal::Decimal) {
        if self.is_triggered {
            return; // Don't update while triggered
        }
        self.set_reference_price(price);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_circuit_breaker_creation() {
        let cb = CircuitBreaker::new(500, 60); // 5% band, 60s cooldown
        assert_eq!(cb.price_band_bps, 500);
        assert!(!cb.is_triggered);
    }

    #[test]
    fn test_price_band_validation() {
        let mut cb = CircuitBreaker::new(500, 60); // 5% band
        cb.set_reference_price(dec!(100.00));

        // Should be within bands (100 +/- 5 = 95 to 105)
        assert!(cb.is_within_bands(dec!(100.00)));
        assert!(cb.is_within_bands(dec!(95.00)));
        assert!(cb.is_within_bands(dec!(105.00)));

        // Should be outside bands
        assert!(!cb.is_within_bands(dec!(94.99)));
        assert!(!cb.is_within_bands(dec!(105.01)));
    }

    #[test]
    fn test_circuit_breaker_trigger() {
        let mut cb = CircuitBreaker::new(500, 60);
        cb.set_reference_price(dec!(100.00));

        cb.trigger();
        assert!(cb.is_triggered);
        assert!(!cb.is_within_bands(dec!(100.00))); // Nothing allowed when triggered
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new(500, 60);
        cb.trigger();
        assert!(cb.is_triggered);

        cb.reset();
        assert!(!cb.is_triggered);
    }

    #[test]
    fn test_validate_price() {
        let mut cb = CircuitBreaker::new(500, 60);
        cb.set_reference_price(dec!(100.00));

        assert!(cb.validate_price(dec!(100.00)).is_ok());
        assert!(cb.validate_price(dec!(95.00)).is_ok());
        assert!(cb.validate_price(dec!(105.00)).is_ok());

        let result = cb.validate_price(dec!(110.00));
        assert!(result.is_err());
    }
}
