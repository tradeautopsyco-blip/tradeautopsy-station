/// Maps Zerodha segment/exchange to Kotak Neo exchangeSegment string
/// Kotak values: nse_cm, bse_cm, nse_fo, bse_fo, cde_fo, mcx_fo
pub fn to_kotak_segment(exchange: &str, segment: &str) -> &'static str {
    match (exchange, segment) {
        ("NSE", "NSE-EQ") | ("NSE", "NSE") => "nse_cm",
        ("BSE", "BSE-EQ") | ("BSE", "BSE") => "bse_cm",
        ("NSE", "NSE-FO") | (_, "NFO") => "nse_fo",
        ("BSE", "BSE-FO") | (_, "BFO") => "bse_fo",
        ("NSE", "CDS") | (_, "CDS") => "cde_fo",
        ("MCX", _) => "mcx_fo",
        _ => "nse_cm",
    }
}

/// Builds Kotak LTP path segment: "{exchangeSegment}|{tradingSymbol}"
pub fn kotak_ltp_key(exchange: &str, segment: &str, trading_symbol: &str) -> String {
    format!(
        "{}|{}",
        to_kotak_segment(exchange, segment),
        trading_symbol
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nse_equity_maps_to_nse_cm() {
        assert_eq!(to_kotak_segment("NSE", "NSE"), "nse_cm");
        assert_eq!(kotak_ltp_key("NSE", "NSE", "RELIANCE"), "nse_cm|RELIANCE");
    }
}
