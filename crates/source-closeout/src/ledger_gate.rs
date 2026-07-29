//! Reject hedged / fake ledger success strings.

/// True when `result` must not be written to the session ledger.
pub fn ledger_result_blocked(result: &str) -> Option<&'static str> {
    let r = result.trim();
    if r.is_empty() {
        return Some("ledger result empty");
    }
    let low = r.to_lowercase();
    // Hedged "success" used by broken batch scripts (2026-07-29 schemeless migrate).
    const BAD: &[&str] = &[
        "或见上",
        "见上",
        "见 diagnose",
        "见上条",
        "成功或见",
        "or see above",
        "see above",
        "verify_ok:false",
        "校验成功或",
    ];
    for b in BAD {
        if r.contains(b) || low.contains(&b.to_lowercase()) {
            return Some(
                "hedged ledger result banned — write exact 校验成功 / skip:… / fail:… \
                 (never 「校验成功或见上」)",
            );
        }
    }
    // Ambiguous success that is not the device-verify phrase.
    if (low.contains("success") || r.contains("成功"))
        && !r.contains("校验成功")
        && !r.starts_with("skip:")
        && !r.starts_with("fail:")
        && !r.starts_with("disable:")
        && !r.starts_with("fixed")
    {
        // allow migrate/hunt intermediate notes without 成功
        if r.contains("成功") {
            return Some(
                "ambiguous 成功 without 校验成功 — device verify must use exact 「校验成功」",
            );
        }
    }
    None
}

pub fn gate_ledger_result(result: &str) -> Result<(), String> {
    match ledger_result_blocked(result) {
        Some(msg) => Err(msg.into()),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_hedged_success() {
        assert!(gate_ledger_result("校验成功或见上").is_err());
        assert!(gate_ledger_result("校验成功").is_ok());
        assert!(gate_ledger_result("skip:l2_http_dead").is_ok());
        assert!(gate_ledger_result("fail:搜索失效").is_ok());
    }
}
