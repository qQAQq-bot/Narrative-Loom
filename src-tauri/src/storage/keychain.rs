//! OS Keychain integration for secure API key storage.
//!
//! Uses the `keyring` crate to store API keys in:
//! - Windows: Credential Manager
//! - macOS: Keychain
//! - Linux: Secret Service (libsecret)

use keyring::Entry;
use thiserror::Error;

/// Service name for keychain entries
const SERVICE_NAME: &str = "narrative-loom";

/// Keychain-related errors
#[derive(Debug, Error)]
pub enum KeychainError {
    #[error("Failed to access keychain: {0}")]
    AccessError(String),

    #[error("Key not found for provider: {0}")]
    NotFound(String),

    #[error("Failed to store key: {0}")]
    StoreError(String),

    #[error("Failed to delete key: {0}")]
    DeleteError(String),
}

/// Keychain service for secure API key storage
pub struct KeychainService;

impl KeychainService {
    /// Create a new keychain service instance
    pub fn new() -> Self {
        Self
    }

    /// Build the keychain entry key from provider ID
    fn entry_key(provider_id: &str) -> String {
        format!("api-key-{}", provider_id)
    }

    /// Get a keyring entry for the given provider
    fn get_entry(provider_id: &str) -> Result<Entry, KeychainError> {
        let key = Self::entry_key(provider_id);
        Entry::new(SERVICE_NAME, &key)
            .map_err(|e| KeychainError::AccessError(e.to_string()))
    }

    /// Store an API key for a provider
    pub fn store_key(&self, provider_id: &str, api_key: &str) -> Result<(), KeychainError> {
        let entry = Self::get_entry(provider_id)?;
        entry
            .set_password(api_key)
            .map_err(|e| KeychainError::StoreError(e.to_string()))
    }

    /// Retrieve an API key for a provider
    pub fn get_key(&self, provider_id: &str) -> Result<String, KeychainError> {
        let entry = Self::get_entry(provider_id)?;
        entry
            .get_password()
            .map_err(|e| match e {
                keyring::Error::NoEntry => KeychainError::NotFound(provider_id.to_string()),
                _ => KeychainError::AccessError(e.to_string()),
            })
    }

    /// Delete an API key for a provider
    pub fn delete_key(&self, provider_id: &str) -> Result<(), KeychainError> {
        let entry = Self::get_entry(provider_id)?;
        entry
            .delete_password()
            .map_err(|e| match e {
                keyring::Error::NoEntry => KeychainError::NotFound(provider_id.to_string()),
                _ => KeychainError::DeleteError(e.to_string()),
            })
    }

    /// Check if an API key exists for a provider
    pub fn has_key(&self, provider_id: &str) -> bool {
        self.get_key(provider_id).is_ok()
    }

    /// Get a masked version of the API key (for display)
    /// Shows first 4 and last 4 characters
    pub fn get_masked_key(&self, provider_id: &str) -> Result<String, KeychainError> {
        let key = self.get_key(provider_id)?;
        Ok(mask_api_key(&key))
    }
}

impl Default for KeychainService {
    fn default() -> Self {
        Self::new()
    }
}

/// Mask an API key for display purposes
/// Shows first 4 and last 4 characters with dots in between
pub fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "********".to_string();
    }
    
    let prefix = &key[..4];
    let suffix = &key[key.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("sk-1234567890abcdef"), "sk-1...cdef");
        assert_eq!(mask_api_key("short"), "********");
        assert_eq!(mask_api_key("12345678"), "********");
        assert_eq!(mask_api_key("123456789"), "1234...6789");
    }

    #[test]
    fn test_entry_key() {
        assert_eq!(
            KeychainService::entry_key("openai"),
            "api-key-openai"
        );
        assert_eq!(
            KeychainService::entry_key("deepseek"),
            "api-key-deepseek"
        );
    }
}
