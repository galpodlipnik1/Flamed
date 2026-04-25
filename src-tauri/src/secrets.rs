use crate::{
    error::{AppError, AppResult},
    settings::AiProvider,
};
use keyring::{Entry, Error as KeyringError};

const SERVICE: &str = "com.flamed.flamed";

pub trait SecretStore {
    fn get_api_key(&self, provider: AiProvider) -> AppResult<Option<String>>;
    fn set_api_key(&self, provider: AiProvider, api_key: &str) -> AppResult<()>;
    fn clear_api_key(&self, provider: AiProvider) -> AppResult<()>;

    fn has_api_key(&self, provider: AiProvider) -> AppResult<bool> {
        Ok(self.get_api_key(provider)?.is_some())
    }
}

pub struct KeyringSecretStore;

impl KeyringSecretStore {
    fn entry(&self, provider: AiProvider) -> AppResult<Entry> {
        Entry::new(SERVICE, &format!("{}_api_key", provider.as_key()))
            .map_err(|error| AppError::Secret(error.to_string()))
    }
}

impl SecretStore for KeyringSecretStore {
    fn get_api_key(&self, provider: AiProvider) -> AppResult<Option<String>> {
        match self.entry(provider)?.get_password() {
            Ok(api_key) if api_key.trim().is_empty() => Ok(None),
            Ok(api_key) => Ok(Some(api_key)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(AppError::Secret(error.to_string())),
        }
    }

    fn set_api_key(&self, provider: AiProvider, api_key: &str) -> AppResult<()> {
        let api_key = api_key.trim();

        if api_key.is_empty() {
            return Err(AppError::Secret(format!(
                "{} API key is empty.",
                provider.label()
            )));
        }

        let entry = self.entry(provider)?;
        entry
            .set_password(api_key)
            .map_err(|error| AppError::Secret(error.to_string()))?;

        match entry.get_password() {
            Ok(saved) if saved.trim().is_empty() => Err(AppError::Secret(format!(
                "{} API key could not be read after saving.",
                provider.label()
            ))),
            Ok(_) => Ok(()),
            Err(error) => Err(AppError::Secret(format!(
                "{} API key could not be read after saving: {error}",
                provider.label()
            ))),
        }
    }

    fn clear_api_key(&self, provider: AiProvider) -> AppResult<()> {
        match self.entry(provider)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(AppError::Secret(error.to_string())),
        }
    }
}
