//! API keys and tokens, kept in the OS keychain.
//!
//! Environment variables (`RESUMAKER_<PROVIDER>_KEY`) take precedence, which suits CI
//! and systems without a keychain.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::Error;
use crate::provider::ProviderKind;

const SERVICE: &str = "resumaker";

/// What a secret belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKey<'a> {
    Provider(ProviderKind),
    /// Bearer token for an API source, identified by its URL.
    Api(&'a str),
}

impl SecretKey<'_> {
    fn account(&self) -> String {
        match self {
            Self::Provider(p) => format!("provider:{}", p.id()),
            Self::Api(url) => format!("api:{url}"),
        }
    }

    fn env_var(&self) -> Option<String> {
        match self {
            Self::Provider(p) => Some(p.key_env_var()),
            Self::Api(_) => None,
        }
    }
}

/// Where secrets are stored.
pub trait SecretStore: Send + Sync {
    fn get(&self, account: &str) -> Result<Option<String>, Error>;
    fn set(&self, account: &str, secret: &str) -> Result<(), Error>;
    /// Returns whether a secret was deleted.
    fn delete(&self, account: &str) -> Result<bool, Error>;
}

/// The platform keychain: Windows Credential Manager, macOS Keychain or the
/// Secret Service on Linux.
#[derive(Debug, Default)]
pub struct Keychain;

impl Keychain {
    fn entry(account: &str) -> Result<keyring::Entry, Error> {
        keyring::Entry::new(SERVICE, account).map_err(keychain_error)
    }
}

impl SecretStore for Keychain {
    fn get(&self, account: &str) -> Result<Option<String>, Error> {
        match Self::entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(keychain_error(e)),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), Error> {
        Self::entry(account)?
            .set_password(secret)
            .map_err(keychain_error)
    }

    fn delete(&self, account: &str) -> Result<bool, Error> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(keychain_error(e)),
        }
    }
}

fn keychain_error(e: keyring::Error) -> Error {
    Error::Keychain(e.to_string())
}

/// An in-memory store, for tests and front ends that manage secrets themselves.
#[derive(Debug, Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl SecretStore for MemoryStore {
    fn get(&self, account: &str) -> Result<Option<String>, Error> {
        Ok(self.0.lock().expect("poisoned").get(account).cloned())
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), Error> {
        self.0
            .lock()
            .expect("poisoned")
            .insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn delete(&self, account: &str) -> Result<bool, Error> {
        Ok(self.0.lock().expect("poisoned").remove(account).is_some())
    }
}

/// Reads and writes secrets, checking environment variables first.
pub struct Secrets {
    store: Box<dyn SecretStore>,
}

impl Default for Secrets {
    fn default() -> Self {
        Self::new(Keychain)
    }
}

impl Secrets {
    pub fn new(store: impl SecretStore + 'static) -> Self {
        Self {
            store: Box::new(store),
        }
    }

    pub fn get(&self, key: SecretKey<'_>) -> Result<Option<String>, Error> {
        if let Some(value) = key
            .env_var()
            .and_then(|v| std::env::var(v).ok())
            .filter(|v| !v.trim().is_empty())
        {
            return Ok(Some(value));
        }
        self.store.get(&key.account())
    }

    /// Where a secret comes from, without revealing it.
    pub fn origin(&self, key: SecretKey<'_>) -> Result<Option<SecretOrigin>, Error> {
        if let Some(var) = key
            .env_var()
            .filter(|v| std::env::var(v).is_ok_and(|v| !v.trim().is_empty()))
        {
            return Ok(Some(SecretOrigin::Env(var)));
        }
        Ok(self
            .store
            .get(&key.account())?
            .map(|_| SecretOrigin::Keychain))
    }

    pub fn set(&self, key: SecretKey<'_>, secret: &str) -> Result<(), Error> {
        let secret = secret.trim();
        if secret.is_empty() {
            return Err(Error::Keychain("refusing to store an empty secret".into()));
        }
        self.store.set(&key.account(), secret)
    }

    pub fn delete(&self, key: SecretKey<'_>) -> Result<bool, Error> {
        self.store.delete(&key.account())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretOrigin {
    Env(String),
    Keychain,
}

/// Masks a secret for display: `sk-abc…wxyz`.
pub fn mask(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= 10 {
        return "•".repeat(chars.len().max(4));
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_deletes() {
        let secrets = Secrets::new(MemoryStore::default());
        let key = SecretKey::Provider(ProviderKind::Anthropic);
        assert_eq!(secrets.get(key).unwrap(), None);
        secrets.set(key, "  sk-test  ").unwrap();
        assert_eq!(secrets.get(key).unwrap().as_deref(), Some("sk-test"));
        assert_eq!(secrets.origin(key).unwrap(), Some(SecretOrigin::Keychain));
        assert!(secrets.delete(key).unwrap());
        assert!(!secrets.delete(key).unwrap());
    }

    #[test]
    fn api_tokens_are_keyed_by_url() {
        let secrets = Secrets::new(MemoryStore::default());
        secrets.set(SecretKey::Api("https://a"), "t1").unwrap();
        assert_eq!(secrets.get(SecretKey::Api("https://b")).unwrap(), None);
        assert_eq!(
            secrets.get(SecretKey::Api("https://a")).unwrap().as_deref(),
            Some("t1")
        );
    }

    #[test]
    fn rejects_empty_secret() {
        let secrets = Secrets::new(MemoryStore::default());
        assert!(
            secrets
                .set(SecretKey::Provider(ProviderKind::Openai), " ")
                .is_err()
        );
    }

    #[test]
    fn masks_secrets() {
        assert_eq!(mask("sk-1234567890abcd"), "sk-1…abcd");
        assert_eq!(mask("short"), "•••••");
    }
}
