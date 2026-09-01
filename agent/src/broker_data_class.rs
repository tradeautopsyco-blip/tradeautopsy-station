//! Per-data-class freshness and completeness (Backend Box v1 #16/#17).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerDataClass {
    FillsTradeHistory,
    BalancesHoldings,
    OpenOrders,
}

impl BrokerDataClass {
    pub const ALL: [Self; 3] = [
        Self::FillsTradeHistory,
        Self::BalancesHoldings,
        Self::OpenOrders,
    ];

    pub fn wire_key(self) -> &'static str {
        match self {
            Self::FillsTradeHistory => "fills_trade_history",
            Self::BalancesHoldings => "balances_holdings",
            Self::OpenOrders => "open_orders",
        }
    }
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct DataClassFreshness {
    pub current: bool,
    pub last_success_at_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limited_until_ms: Option<i64>,
    pub requires_manual_retry: bool,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct BrokerDataClassCompleteness {
    pub fills_trade_history: DataClassFreshness,
    pub balances_holdings: DataClassFreshness,
    pub open_orders: DataClassFreshness,
}

impl BrokerDataClassCompleteness {
    pub fn get_mut(&mut self, class: BrokerDataClass) -> &mut DataClassFreshness {
        match class {
            BrokerDataClass::FillsTradeHistory => &mut self.fills_trade_history,
            BrokerDataClass::BalancesHoldings => &mut self.balances_holdings,
            BrokerDataClass::OpenOrders => &mut self.open_orders,
        }
    }

    pub fn get(&self, class: BrokerDataClass) -> &DataClassFreshness {
        match class {
            BrokerDataClass::FillsTradeHistory => &self.fills_trade_history,
            BrokerDataClass::BalancesHoldings => &self.balances_holdings,
            BrokerDataClass::OpenOrders => &self.open_orders,
        }
    }

    pub fn all_current(&self) -> bool {
        BrokerDataClass::ALL.iter().all(|c| self.get(*c).current)
    }

    pub fn any_rate_limited(&self, now_ms: i64) -> bool {
        BrokerDataClass::ALL.iter().any(|c| {
            self.get(*c)
                .rate_limited_until_ms
                .is_some_and(|until| until > now_ms)
        })
    }

    pub fn earliest_rate_limit_retry_ms(&self, now_ms: i64) -> Option<i64> {
        BrokerDataClass::ALL
            .iter()
            .filter_map(|c| {
                let until = self.get(*c).rate_limited_until_ms?;
                if until > now_ms {
                    Some(until)
                } else {
                    None
                }
            })
            .min()
    }

    pub fn any_requires_manual_retry(&self) -> bool {
        BrokerDataClass::ALL
            .iter()
            .any(|c| self.get(*c).requires_manual_retry)
    }

    pub fn failing_class_labels(&self) -> Vec<&'static str> {
        BrokerDataClass::ALL
            .iter()
            .filter(|c| !self.get(**c).current)
            .map(|c| c.wire_key())
            .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BrokerBalancesSnapshot {
    pub holdings: Vec<BrokerHolding>,
    pub unrealized_pnl: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrokerHolding {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BrokerOpenOrdersSnapshot {
    pub orders: Vec<BrokerOpenOrder>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrokerOpenOrder {
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub qty: f64,
    pub price: Option<f64>,
    pub product: Option<String>,
    pub exchange_segment: Option<String>,
    pub status: Option<String>,
    pub unfilled_qty: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrokerPortfolioHolding {
    pub symbol: String,
    pub exchange_segment: String,
    pub quantity: f64,
    pub sellable_quantity: f64,
    pub average_price: f64,
    pub market_value: f64,
    pub instrument_type: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BrokerHoldingsSnapshot {
    pub holdings: Vec<BrokerPortfolioHolding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrokerPositionRow {
    pub symbol: String,
    pub exchange_segment: String,
    pub product: String,
    pub net_qty: f64,
    pub trading_symbol: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BrokerPositionsSnapshot {
    pub positions: Vec<BrokerPositionRow>,
}

impl BrokerOpenOrdersSnapshot {
    pub fn empty() -> Self {
        Self { orders: vec![] }
    }
}
