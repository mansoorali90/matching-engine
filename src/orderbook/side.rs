use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "Buy"),
            Side::Sell => write!(f, "Sell"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    /// Execute immediately at best available price
    Market,
    /// Execute at specified price or better
    Limit,
    /// Immediate-or-Cancel: execute immediately, cancel remaining
    IOC,
    /// Fill-or-Kill: execute entirely or cancel completely
    FOK,
}

impl std::fmt::Display for OrderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderType::Market => write!(f, "Market"),
            OrderType::Limit => write!(f, "Limit"),
            OrderType::IOC => write!(f, "IOC"),
            OrderType::FOK => write!(f, "FOK"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimeInForce {
    /// Good-Til-Cancelled: remains active until cancelled
    GTC,
    /// Immediate-or-Cancel: execute immediately, cancel remaining
    IOC,
    /// Fill-or-Kill: execute entirely or cancel completely
    FOK,
    /// Good-Til-Date: remains active until specified time
    GTD,
}

impl std::fmt::Display for TimeInForce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeInForce::GTC => write!(f, "GTC"),
            TimeInForce::IOC => write!(f, "IOC"),
            TimeInForce::FOK => write!(f, "FOK"),
            TimeInForce::GTD => write!(f, "GTD"),
        }
    }
}
