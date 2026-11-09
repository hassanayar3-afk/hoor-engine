use std::collections::HashMap;
use std::env;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

const ALERT_COOLDOWN: Duration = Duration::from_secs(15 * 60);

static LAST_MANUAL_ALERT: LazyLock<Mutex<HashMap<String, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn telegram_token() -> Option<String> {
    env::var("HOOR_TELEGRAM_BOT_TOKEN")
        .ok()
        .or_else(|| env::var("TELEGRAM_BOT_TOKEN").ok())
}

fn telegram_chat_id() -> Option<String> {
    env::var("HOOR_TELEGRAM_CHAT_ID")
        .ok()
        .or_else(|| env::var("TELEGRAM_CHAT_ID").ok())
}

fn cooldown_allows(symbol: &str) -> bool {
    let map = LAST_MANUAL_ALERT.lock();
    match map.get(symbol) {
        Some(last) => last.elapsed() >= ALERT_COOLDOWN,
        None => true,
    }
}

fn send_telegram_text(text: &str) -> bool {
    let Some(token) = telegram_token() else {
        eprintln!("TELEGRAM: bot token missing (HOOR_TELEGRAM_BOT_TOKEN / TELEGRAM_BOT_TOKEN)");
        return false;
    };
    let Some(chat_id) = telegram_chat_id() else {
        eprintln!("TELEGRAM: chat id missing (HOOR_TELEGRAM_CHAT_ID / TELEGRAM_CHAT_ID)");
        return false;
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("TELEGRAM client: {e}");
            return false;
        }
    };

    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    match client
        .post(url)
        .json(&serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        }))
        .send()
    {
        Ok(resp) if resp.status().is_success() => {
            eprintln!("TELEGRAM: message sent");
            true
        }
        Ok(resp) => {
            eprintln!("TELEGRAM: HTTP {}", resp.status());
            false
        }
        Err(e) => {
            eprintln!("TELEGRAM: {e}");
            false
        }
    }
}

/// Critical alert: no cooldown.
pub fn send_critical_alert(kind: &str, detail: &str) -> bool {
    let text = format!("HOOR CRITICAL\nKind: {kind}\n{detail}");
    send_telegram_text(&text)
}

pub fn notify_emergency_halt(reason: &str) -> bool {
    send_critical_alert("EMERGENCY_HALT", reason)
}

pub fn notify_stream_freeze(detail: &str) -> bool {
    send_critical_alert("STREAM_FREEZE", detail)
}

pub fn notify_exchange_failure_streak(streak: u32, err: &str) -> bool {
    send_critical_alert(
        "EXCHANGE_FAILURE_STREAK",
        &format!("Consecutive failures: {streak}\n{err}"),
    )
}

/// Manual 80-84 band alert: 15 minute cooldown per symbol.
pub fn send_manual_alert(symbol: &str, consensus: u32, change_pct: f64) -> bool {
    if !cooldown_allows(symbol) {
        return false;
    }

    let pct = consensus as f64 / 1_000_000.0;
    let text = format!(
        "HOOR MANUAL ALERT\nSymbol: {symbol}\nConsensus: {pct:.1}%\n24h: {change_pct:.2}%\nBand 80-84 — manual only."
    );

    let sent = send_telegram_text(&text);
    if sent {
        LAST_MANUAL_ALERT
            .lock()
            .insert(symbol.to_string(), Instant::now());
    }
    sent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_blocks_second_immediate_send() {
        let sym = "TESTCOOLDOWN_SYM";
        LAST_MANUAL_ALERT
            .lock()
            .insert(sym.to_string(), Instant::now());
        assert!(!cooldown_allows(sym));
    }

    #[test]
    fn critical_alert_skips_manual_cooldown_map() {
        let sym = "TESTCOOLDOWN_SYM";
        assert!(!cooldown_allows(sym));
        let _ = send_critical_alert("TEST", "unit test without network credentials");
    }
}
