//! The DSSE envelope (<https://github.com/secure-systems-lab/dsse>): a
//! payload, its type, and signatures over the pre-authentication
//! encoding of the two, so a signature never verifies a payload read as
//! another type.

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// The payload type of an in-toto statement.
pub const IN_TOTO: &str = "application/vnd.in-toto+json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    /// The payload, base64.
    pub payload: String,
    pub payload_type: String,
    pub signatures: Vec<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    /// Which key signed it, as the signer names it; empty when unnamed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub keyid: String,
    /// The signature, base64.
    pub sig: String,
}

/// `PAE(type, body)`: what a DSSE signature signs.
pub fn pae(payload_type: &str, payload: &[u8]) -> Vec<u8> {
    let mut out = format!(
        "DSSEv1 {} {} {} ",
        payload_type.len(),
        payload_type,
        payload.len()
    )
    .into_bytes();
    out.extend_from_slice(payload);
    out
}

impl Envelope {
    /// An envelope of `payload`, unsigned.
    pub fn new(payload_type: &str, payload: &[u8]) -> Envelope {
        Envelope {
            payload: STANDARD.encode(payload),
            payload_type: payload_type.into(),
            signatures: Vec::new(),
        }
    }

    /// The payload, decoded.
    pub fn body(&self) -> Result<Vec<u8>> {
        STANDARD
            .decode(&self.payload)
            .context("the envelope's payload is not base64")
    }

    /// What a signature over this envelope signs.
    pub fn signed_bytes(&self) -> Result<Vec<u8>> {
        Ok(pae(&self.payload_type, &self.body()?))
    }

    /// Add a signature, its bytes raw.
    pub fn sign_with(&mut self, keyid: &str, sig: &[u8]) {
        self.signatures.push(Signature {
            keyid: keyid.into(),
            sig: STANDARD.encode(sig),
        });
    }

    /// The payload, when it is of `payload_type`.
    pub fn payload_of(&self, payload_type: &str) -> Result<Vec<u8>> {
        if self.payload_type != payload_type {
            bail!(
                "the envelope holds {}, not {payload_type}",
                self.payload_type
            );
        }
        self.body()
    }
}

impl Signature {
    pub fn bytes(&self) -> Result<Vec<u8>> {
        STANDARD
            .decode(&self.sig)
            .context("a signature is not base64")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The specification's own example: `PAE("http://example.com/HelloWorld",
    /// "hello world")`.
    #[test]
    fn the_pre_authentication_encoding_is_the_specifications() {
        assert_eq!(
            pae("http://example.com/HelloWorld", b"hello world"),
            b"DSSEv1 29 http://example.com/HelloWorld 11 hello world".to_vec()
        );
        assert_eq!(pae("", b""), b"DSSEv1 0  0 ".to_vec());
    }

    #[test]
    fn an_envelope_round_trips_and_names_its_type() {
        let mut e = Envelope::new(IN_TOTO, b"{\"a\":1}");
        e.sign_with("k", b"sig");
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("\"payloadType\":\"application/vnd.in-toto+json\""));
        let back: Envelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
        assert_eq!(back.payload_of(IN_TOTO).unwrap(), b"{\"a\":1}");
        assert!(back.payload_of("text/plain").is_err());
        assert_eq!(back.signatures[0].bytes().unwrap(), b"sig");
        assert_eq!(back.signed_bytes().unwrap(), pae(IN_TOTO, b"{\"a\":1}"));
    }
}
