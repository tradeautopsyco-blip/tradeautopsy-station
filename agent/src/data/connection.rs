//! Persistent connected-broker runtime (R0 contract).

#![allow(dead_code)]

use super::host_policy::AuthMode;
use super::identity::Family;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerConnectionRuntime {
    pub adapter_id: String,
    /// Keychain handle id. Never the raw secret.
    pub credential_handle: Option<String>,
    pub shared_budget: u32,
    pub subscriptions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetError {
    Exhausted,
}

impl BrokerConnectionRuntime {
    /// Start-time handle only — never a raw API key/secret.
    pub fn on_start(
        adapter_id: impl Into<String>,
        credential_handle: Option<String>,
        shared_budget: u32,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            credential_handle,
            shared_budget,
            subscriptions: vec![],
        }
    }

    pub fn subscribe(&mut self, instrument_id: &str) {
        let id = instrument_id.trim().to_ascii_lowercase();
        if id.is_empty() {
            return;
        }
        if !self.subscriptions.iter().any(|existing| existing == &id) {
            self.subscriptions.push(id);
        }
    }

    pub fn unsubscribe(&mut self, instrument_id: &str) {
        let id = instrument_id.trim().to_ascii_lowercase();
        self.subscriptions.retain(|existing| existing != &id);
    }

    pub fn debit(&mut self, _family: Family, cost: u32) -> Result<(), BudgetError> {
        if self.shared_budget < cost {
            return Err(BudgetError::Exhausted);
        }
        self.shared_budget -= cost;
        Ok(())
    }

    pub fn attach_auth(&self, auth_mode: AuthMode) -> Option<&str> {
        match auth_mode {
            AuthMode::Public => None,
            AuthMode::PrivateRead => self.credential_handle.as_deref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_and_account_share_one_budget() {
        let mut runtime = BrokerConnectionRuntime {
            adapter_id: "binance_com".into(),
            credential_handle: Some("keychain:binance".into()),
            shared_budget: 5,
            subscriptions: vec!["btcusdt".into()],
        };
        runtime.debit(Family::Market, 2).unwrap();
        runtime.debit(Family::Account, 3).unwrap();
        assert_eq!(
            runtime.debit(Family::Market, 1),
            Err(BudgetError::Exhausted)
        );
    }

    #[test]
    fn public_calls_do_not_receive_the_credential_handle() {
        let runtime = BrokerConnectionRuntime {
            adapter_id: "binance_com".into(),
            credential_handle: Some("keychain:binance".into()),
            shared_budget: 5,
            subscriptions: vec![],
        };
        assert_eq!(runtime.attach_auth(AuthMode::Public), None);
        assert_eq!(
            runtime.attach_auth(AuthMode::PrivateRead),
            Some("keychain:binance")
        );
    }

    #[test]
    fn on_start_stores_a_handle_not_a_secret() {
        let runtime = BrokerConnectionRuntime::on_start(
            "binance_com",
            Some("keychain:in.tradeautopsy.station.broker-credentials:conn-1".into()),
            6000,
        );
        assert_eq!(runtime.adapter_id, "binance_com");
        assert_eq!(runtime.shared_budget, 6000);
        let handle = runtime.credential_handle.as_deref().unwrap();
        assert!(handle.starts_with("keychain:"));
        assert!(!handle.contains("secret"));
        assert!(!handle.contains("apiKey"));
    }

    #[test]
    fn subscribe_normalizes_and_dedupes() {
        let mut runtime = BrokerConnectionRuntime::on_start("binance_com", None, 60);
        runtime.subscribe("BTCUSDT");
        runtime.subscribe("btcusdt");
        runtime.subscribe(" ETHUSDT ");
        assert_eq!(runtime.subscriptions, vec!["btcusdt", "ethusdt"]);
        runtime.unsubscribe("ETHUSDT");
        assert_eq!(runtime.subscriptions, vec!["btcusdt"]);
    }
}
