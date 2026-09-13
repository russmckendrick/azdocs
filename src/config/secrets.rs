//! Secrets stay behind this boundary so offline config reads never unlock a keychain.
use crate::error::ConfigError;

pub trait SecretStore {
    fn get(&self, reference: &str) -> Result<String, ConfigError>;
    fn set(&self, reference: &str, secret: &str) -> Result<(), ConfigError>;
    fn delete(&self, reference: &str) -> Result<(), ConfigError>;
}

pub struct NativeSecretStore;

impl SecretStore for NativeSecretStore {
    fn get(&self, reference: &str) -> Result<String, ConfigError> {
        keyring::Entry::new("azdocs", reference)
            .and_then(|entry| entry.get_password())
            .map_err(|_| ConfigError::Secret("could not read the saved secret; unlock the OS credential store or use an environment reference".into()))
    }
    fn set(&self, reference: &str, secret: &str) -> Result<(), ConfigError> {
        keyring::Entry::new("azdocs", reference)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|_| {
                ConfigError::Secret("could not save the secret in the OS credential store".into())
            })
    }
    fn delete(&self, reference: &str) -> Result<(), ConfigError> {
        keyring::Entry::new("azdocs", reference)
            .and_then(|entry| entry.delete_credential())
            .map_err(|_| ConfigError::Secret("could not remove the saved secret".into()))
    }
}
