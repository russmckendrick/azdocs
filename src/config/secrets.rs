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
            .map_err(|err| ConfigError::Secret(format!("could not read the saved secret ({err}); unlock the OS credential store or use an environment reference")))
    }
    fn set(&self, reference: &str, secret: &str) -> Result<(), ConfigError> {
        keyring::Entry::new("azdocs", reference)
            .and_then(|entry| entry.set_password(secret))
            .map_err(|err| {
                ConfigError::Secret(format!(
                    "could not save the secret in the OS credential store ({err}); use --secret-env or an environment reference instead"
                ))
            })
    }
    fn delete(&self, reference: &str) -> Result<(), ConfigError> {
        keyring::Entry::new("azdocs", reference)
            .and_then(|entry| entry.delete_credential())
            .map_err(|err| {
                ConfigError::Secret(format!("could not remove the saved secret ({err})"))
            })
    }
}
