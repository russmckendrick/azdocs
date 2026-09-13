use azdocs::config::secrets::{NativeSecretStore, SecretStore};
#[test]
#[ignore = "requires an unlocked OS credential store; CI runs this separately"]
fn unit_native_credential_store_round_trips_an_isolated_temporary_entry() {
    let reference = uuid::Uuid::new_v4().to_string();
    NativeSecretStore
        .set(&reference, "azdocs-credential-smoke-not-a-real-secret")
        .unwrap();
    let read = NativeSecretStore.get(&reference);
    let deleted = NativeSecretStore.delete(&reference);
    assert_eq!(read.unwrap(), "azdocs-credential-smoke-not-a-real-secret");
    assert!(deleted.is_ok());
    assert!(NativeSecretStore.get(&reference).is_err());
}
