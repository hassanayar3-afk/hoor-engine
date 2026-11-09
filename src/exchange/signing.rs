use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn sign_query(secret: &str, query: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(query.as_bytes());
    let result = mac.finalize().into_bytes();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_hex() {
        let sig = sign_query("secret", "symbol=BTCUSDT&side=BUY");
        assert!(!sig.is_empty());
        assert_eq!(sig.len(), 64);
    }
}
