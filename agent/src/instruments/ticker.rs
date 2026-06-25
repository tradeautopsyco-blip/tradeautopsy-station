/// NSE wire ticker for Bar declare autocomplete — broker token, not display names (#181).
pub fn normalize_broker_ticker(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_uppercase();
    if s.is_empty() {
        return None;
    }

    for suffix in ["-EQ", "-BE", "-SM", "-IL", "-BL", "-N1", "-N2", "-N3", "-N4"] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
            break;
        }
    }
    s = s.trim().to_string();
    if s.is_empty() {
        return None;
    }
    if s.contains(' ') {
        return None;
    }
    if s.len() < 2 || s.len() > 24 {
        return None;
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '&' || c == '.' || c == '-')
    {
        return None;
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_eq_suffix() {
        assert_eq!(normalize_broker_ticker("TCS-EQ").as_deref(), Some("TCS"));
    }

    #[test]
    fn rejects_company_name_with_spaces() {
        assert_eq!(normalize_broker_ticker("RELIANCE INDUSTRY LIMITED"), None);
    }

    #[test]
    fn allows_ampersand_ticker() {
        assert_eq!(normalize_broker_ticker("M&M-EQ").as_deref(), Some("M&M"));
    }
}
