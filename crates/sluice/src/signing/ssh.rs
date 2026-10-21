//! An SSH key's signature over a DSSE envelope (78; ADR 0032): `SSHSIG`
//! over the envelope's pre-authentication encoding, in the `sluice-run`
//! namespace, checked against the allowed-signers lines a signer's
//! policy table names.

use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use ssh_key::{HashAlg, LineEnding, PrivateKey, PublicKey, SshSig};

use super::dsse::Envelope;

/// The namespace every run signature is made in, so a signature made for
/// anything else never verifies as a run's.
pub const NAMESPACE: &str = "sluice-run";

/// One allowed-signers line: who, and their public key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Allowed {
    pub principal: String,
    pub key: PublicKey,
}

/// The lines of an allowed-signers text, `ssh-keygen`'s form:
/// `principals [options] keytype base64 [comment]`. Blank lines and `#`
/// comments are skipped.
pub fn allowed(text: &str) -> Result<Vec<Allowed>> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let at = words
            .iter()
            .position(|w| is_key_type(w))
            .filter(|&i| i >= 1 && i + 1 < words.len())
            .ok_or_else(|| {
                anyhow!(
                    "allowed signers line {}: no principal and key in {line:?}",
                    n + 1
                )
            })?;
        let key = PublicKey::from_openssh(&format!("{} {}", words[at], words[at + 1]))
            .with_context(|| format!("allowed signers line {}", n + 1))?;
        out.push(Allowed {
            principal: words[0].into(),
            key,
        });
    }
    Ok(out)
}

fn is_key_type(word: &str) -> bool {
    word.starts_with("ssh-") || word.starts_with("ecdsa-") || word.starts_with("sk-")
}

/// A private key from its file, unencrypted: sluice signs with no one to
/// type a passphrase.
pub fn private_key(path: &Path) -> Result<PrivateKey> {
    let key = PrivateKey::read_openssh_file(path)
        .with_context(|| format!("reading the signing key {}", path.display()))?;
    if key.is_encrypted() {
        bail!(
            "the signing key {} has a passphrase; sluice signs with none to type",
            path.display()
        );
    }
    Ok(key)
}

/// Sign `envelope` with `key`, naming the key by its fingerprint.
pub fn sign(envelope: &mut Envelope, key: &PrivateKey) -> Result<()> {
    let msg = envelope.signed_bytes()?;
    let sig = key
        .sign(NAMESPACE, HashAlg::Sha512, &msg)
        .context("signing the run's statement")?;
    let keyid = key.public_key().fingerprint(HashAlg::Sha256).to_string();
    envelope.sign_with(&keyid, sig.to_pem(LineEnding::LF)?.as_bytes());
    Ok(())
}

/// The principal of the first allowed signer whose key made one of the
/// envelope's signatures over exactly its payload, in this namespace.
pub fn verify<'a>(envelope: &Envelope, allowed: &'a [Allowed]) -> Result<Option<&'a Allowed>> {
    let msg = envelope.signed_bytes()?;
    for s in &envelope.signatures {
        let Ok(bytes) = s.bytes() else { continue };
        let Ok(pem) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let Ok(sig) = SshSig::from_pem(pem) else {
            continue;
        };
        for a in allowed {
            if a.key.key_data() == sig.public_key() && a.key.verify(NAMESPACE, &msg, &sig).is_ok() {
                return Ok(Some(a));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::dsse::IN_TOTO;
    use ssh_key::Algorithm;

    fn key() -> PrivateKey {
        PrivateKey::random(&mut rand_core::OsRng, Algorithm::Ed25519).unwrap()
    }

    fn line(principal: &str, k: &PrivateKey) -> String {
        format!("{principal} {}", k.public_key().to_openssh().unwrap())
    }

    #[test]
    fn a_listed_key_verifies_and_names_its_principal() {
        let k = key();
        let mut e = Envelope::new(IN_TOTO, b"{\"run\":1}");
        sign(&mut e, &k).unwrap();
        let list = allowed(&format!("# the box\n\n{}\n", line("exec@build-1", &k))).unwrap();
        assert_eq!(
            verify(&e, &list).unwrap().unwrap().principal,
            "exec@build-1"
        );
    }

    #[test]
    fn a_key_not_listed_verifies_nothing() {
        let (k, other) = (key(), key());
        let mut e = Envelope::new(IN_TOTO, b"{\"run\":1}");
        sign(&mut e, &k).unwrap();
        let list = allowed(&line("host-b", &other)).unwrap();
        assert!(verify(&e, &list).unwrap().is_none());
    }

    #[test]
    fn a_tampered_statement_verifies_nothing() {
        let k = key();
        let mut e = Envelope::new(IN_TOTO, b"{\"outcome\":\"fail\"}");
        sign(&mut e, &k).unwrap();
        let list = allowed(&line("box", &k)).unwrap();
        let mut forged = Envelope::new(IN_TOTO, b"{\"outcome\":\"pass\"}");
        forged.signatures = e.signatures.clone();
        assert!(verify(&forged, &list).unwrap().is_none());
        let mut retyped = e.clone();
        retyped.payload_type = "text/plain".into();
        assert!(verify(&retyped, &list).unwrap().is_none());
    }

    #[test]
    fn a_signature_in_another_namespace_verifies_nothing() {
        let k = key();
        let mut e = Envelope::new(IN_TOTO, b"{\"run\":1}");
        let sig = k
            .sign("git", HashAlg::Sha512, &e.signed_bytes().unwrap())
            .unwrap();
        e.sign_with("", sig.to_pem(LineEnding::LF).unwrap().as_bytes());
        let list = allowed(&line("box", &k)).unwrap();
        assert!(verify(&e, &list).unwrap().is_none());
    }

    #[test]
    fn allowed_signers_lines_with_options_and_comments() {
        let k = key();
        let pubkey = k.public_key().to_openssh().unwrap();
        let list = allowed(&format!(
            "a@x,b@x namespaces=\"sluice-run\" {pubkey} the box\n"
        ))
        .unwrap();
        assert_eq!(list[0].principal, "a@x,b@x");
        assert!(allowed("just-a-name\n").is_err());
    }

    #[test]
    fn a_key_file_reads_back_as_its_key() {
        let dir = std::env::temp_dir().join(format!("sluice-ssh-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let k = key();
        let plain = dir.join("plain");
        k.write_openssh_file(&plain, LineEnding::LF).unwrap();
        assert_eq!(private_key(&plain).unwrap().public_key(), k.public_key());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
