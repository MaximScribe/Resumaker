//! Talks to the real OS keychain, so it is skipped by default.
//! Run with `cargo test -p resumaker-core --test keychain -- --ignored`.

use resumaker_core::secrets::{Keychain, SecretStore};

#[test]
#[ignore = "uses the OS keychain"]
fn keychain_round_trip() {
    let account = format!("test:round-trip-{}", std::process::id());
    let store = Keychain;
    assert_eq!(store.get(&account).unwrap(), None);
    store.set(&account, "secret-value").unwrap();
    assert_eq!(
        store.get(&account).unwrap().as_deref(),
        Some("secret-value")
    );
    assert!(store.delete(&account).unwrap());
    assert_eq!(store.get(&account).unwrap(), None);
}
