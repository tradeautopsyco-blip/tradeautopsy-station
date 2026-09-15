//! Persistent connected-broker runtime (R0 contract).

#[cfg(test)]
use super::host_policy::AuthMode;
#[cfg(test)]
use super::identity::Family;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerConnectionRuntime {
    pub adapter_id: String,
    /// Book for this map entry. The connections map may hold the shipping slug
    /// entry and a named book_id entry; never insert nfo/options over the slug key.
    pub book_id: String,
    /// Keychain handle id. Never the raw secret. Do not copy secrets across map entries.
    pub credential_handle: Option<String>,
    /// Stay per map entry. A future fapi book gets its own budget (Binance IP weight is per cluster).
    pub shared_budget: u32,
    pub subscriptions: Vec<String>,
}

#[cfg(test)]
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
        let adapter_id = adapter_id.into();
        let book_id =
            super::source_manifest::shipping_book_id_for_slug(&adapter_id).unwrap_or_default();
        Self {
            adapter_id,
            book_id,
            credential_handle,
            shared_budget,
            subscriptions: vec![],
        }
    }

    /// Named-book handle. Default Start must keep calling [`Self::on_start`] (shipping book).
    pub fn on_start_for_book(
        adapter_id: impl Into<String>,
        book_id: impl Into<String>,
        credential_handle: Option<String>,
        shared_budget: u32,
    ) -> Result<Self, String> {
        let adapter_id = adapter_id.into();
        let book_id = book_id.into();
        let manifest = super::source_manifest::manifest_for_book_id(&book_id)
            .ok_or_else(|| format!("unknown book_id {book_id}"))?;
        if manifest.adapter_id != adapter_id {
            return Err(format!(
                "book {book_id} adapter {} does not match slug {adapter_id}",
                manifest.adapter_id
            ));
        }
        Ok(Self {
            adapter_id,
            book_id: manifest.book_id,
            credential_handle,
            shared_budget,
            subscriptions: vec![],
        })
    }

    pub fn subscribe(&mut self, instrument_id: &str) {
        let id = self.normalize_subscription(instrument_id);
        if id.is_empty() {
            return;
        }
        if !self.subscriptions.iter().any(|existing| existing == &id) {
            self.subscriptions.push(id);
        }
    }

    #[cfg(test)]
    pub fn unsubscribe(&mut self, instrument_id: &str) {
        let id = self.normalize_subscription(instrument_id);
        self.subscriptions.retain(|existing| existing != &id);
    }

    fn normalize_subscription(&self, instrument_id: &str) -> String {
        let trimmed = instrument_id.trim();
        if self.book_id == super::descriptor::BINANCE_COM_OPTIONS_BOOK_ID {
            trimmed.to_string()
        } else {
            trimmed.to_ascii_lowercase()
        }
    }

    #[cfg(test)]
    pub fn debit(&mut self, _family: Family, cost: u32) -> Result<(), BudgetError> {
        if self.shared_budget < cost {
            return Err(BudgetError::Exhausted);
        }
        self.shared_budget -= cost;
        Ok(())
    }

    #[cfg(test)]
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
            book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
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
            book_id: crate::data::BINANCE_COM_SPOT_BOOK_ID.into(),
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
        assert_eq!(runtime.book_id, crate::data::BINANCE_COM_SPOT_BOOK_ID);
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

    #[test]
    fn on_start_ships_cash_not_nfo_for_kotak_neo() {
        let runtime = BrokerConnectionRuntime::on_start("kotak_neo", None, 60);
        assert_eq!(runtime.book_id, crate::data::KOTAK_NSE_BSE_CASH_BOOK_ID);
        assert_ne!(runtime.book_id, crate::data::KOTAK_NSE_NFO_BOOK_ID);
    }

    #[test]
    fn on_start_for_book_binds_named_nfo_after_slug_check() {
        let runtime = BrokerConnectionRuntime::on_start_for_book(
            "kotak_neo",
            crate::data::KOTAK_NSE_NFO_BOOK_ID,
            None,
            60,
        )
        .expect("named NFO book");
        assert_eq!(runtime.adapter_id, "kotak_neo");
        assert_eq!(runtime.book_id, crate::data::KOTAK_NSE_NFO_BOOK_ID);
    }

    #[test]
    fn on_start_for_book_binds_named_options_after_slug_check() {
        let runtime = BrokerConnectionRuntime::on_start_for_book(
            "binance_com",
            crate::data::BINANCE_COM_OPTIONS_BOOK_ID,
            Some("keychain:binance".into()),
            60,
        )
        .expect("named options book");
        assert_eq!(runtime.adapter_id, "binance_com");
        assert_eq!(runtime.book_id, crate::data::BINANCE_COM_OPTIONS_BOOK_ID);
        assert_eq!(runtime.shared_budget, 60);
        let shipping = BrokerConnectionRuntime::on_start("binance_com", None, 6000);
        assert_eq!(shipping.book_id, crate::data::BINANCE_COM_SPOT_BOOK_ID);
        assert_ne!(shipping.book_id, crate::data::BINANCE_COM_OPTIONS_BOOK_ID);
        let usdm = BrokerConnectionRuntime::on_start_for_book(
            "binance_com",
            crate::data::BINANCE_COM_USDM_BOOK_ID,
            None,
            60,
        )
        .expect("named USDM book on the same slug");
        assert_eq!(usdm.book_id, crate::data::BINANCE_COM_USDM_BOOK_ID);
        assert_eq!(usdm.adapter_id, "binance_com");
        assert_ne!(usdm.book_id, shipping.book_id);
    }

    #[test]
    fn options_subscribe_keeps_mixed_case() {
        let mut runtime = BrokerConnectionRuntime::on_start_for_book(
            "binance_com",
            crate::data::BINANCE_COM_OPTIONS_BOOK_ID,
            None,
            60,
        )
        .expect("named options book");
        runtime.subscribe("BTC-200730-9000-C");
        assert_eq!(runtime.subscriptions, vec!["BTC-200730-9000-C"]);
    }

    #[test]
    fn on_start_for_book_refuses_unknown_and_slug_mismatch() {
        let unknown =
            BrokerConnectionRuntime::on_start_for_book("kotak_neo", "binance-com-stocks", None, 60)
                .expect_err("unknown book stays dark");
        assert!(unknown.contains("unknown book_id"), "{unknown}");

        let mismatch = BrokerConnectionRuntime::on_start_for_book(
            "binance_com",
            crate::data::KOTAK_NSE_NFO_BOOK_ID,
            None,
            60,
        )
        .expect_err("COM slug must not bind NFO");
        assert!(mismatch.contains("does not match"), "{mismatch}");
    }
}
