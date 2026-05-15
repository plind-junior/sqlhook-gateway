use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::config::SignatureSpec;
use crate::config::schema::SignatureAlgo;
use crate::error::{AppError, AppResult};

type HmacSha256 = Hmac<Sha256>;

pub fn verify(spec: &SignatureSpec, secret: &[u8], body: &[u8], header_value: Option<&str>) -> AppResult<()> {
    let header_value = header_value.ok_or_else(|| AppError::MissingSignature(spec.header.clone()))?;

    let provided = header_value
        .strip_prefix(spec.prefix.as_str())
        .unwrap_or(header_value)
        .trim();

    let provided_bytes = hex::decode(provided).map_err(|_| AppError::InvalidSignature)?;

    let expected = match spec.algo {
        SignatureAlgo::HmacSha256 => {
            let mut mac = HmacSha256::new_from_slice(secret)
                .expect("HMAC accepts any key length");
            mac.update(body);
            mac.finalize().into_bytes().to_vec()
        }
    };

    if provided_bytes.ct_eq(&expected).into() {
        Ok(())
    } else {
        Err(AppError::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::SignatureAlgo;

    fn spec() -> SignatureSpec {
        SignatureSpec {
            header: "X-Sig".into(),
            algo: SignatureAlgo::HmacSha256,
            secret_env: "X".into(),
            prefix: "sha256=".into(),
        }
    }

    #[test]
    fn round_trip() {
        let secret = b"topsecret";
        let body = b"{\"hello\":\"world\"}";
        let mut mac = HmacSha256::new_from_slice(secret).unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());
        let header = format!("sha256={sig}");
        verify(&spec(), secret, body, Some(&header)).unwrap();
    }

    #[test]
    fn wrong_secret_fails() {
        let body = b"hi";
        let mut mac = HmacSha256::new_from_slice(b"a").unwrap();
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());
        let header = format!("sha256={sig}");
        let err = verify(&spec(), b"b", body, Some(&header)).unwrap_err();
        assert!(matches!(err, AppError::InvalidSignature));
    }

    #[test]
    fn missing_header_fails() {
        let err = verify(&spec(), b"x", b"y", None).unwrap_err();
        assert!(matches!(err, AppError::MissingSignature(_)));
    }
}
