use keyring::Entry;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::LazyLock;

const SERVICE_NAME: &str = "Loop";

// Cache tokens in memory after first keychain read to avoid repeated macOS prompts
static TOKEN_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn entry(key: &str) -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, key).map_err(|e| format!("Keychain error: {}", e))
}

pub fn store_secret(key: &str, value: &str) -> Result<(), String> {
    let entry = entry(key)?;
    entry
        .set_password(value)
        .map_err(|e| format!("Failed to store secret: {}", e))?;
    // Update cache
    TOKEN_CACHE
        .lock()
        .unwrap()
        .insert(key.to_string(), value.to_string());
    Ok(())
}

pub fn get_secret(key: &str) -> Result<Option<String>, String> {
    // Check cache first
    if let Some(val) = TOKEN_CACHE.lock().unwrap().get(key) {
        return Ok(Some(val.clone()));
    }
    // Read from keychain
    let entry = entry(key)?;
    match entry.get_password() {
        Ok(val) => {
            TOKEN_CACHE
                .lock()
                .unwrap()
                .insert(key.to_string(), val.clone());
            Ok(Some(val))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read secret: {}", e)),
    }
}

pub fn delete_secret(key: &str) -> Result<(), String> {
    TOKEN_CACHE.lock().unwrap().remove(key);
    let entry = entry(key)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete secret: {}", e)),
    }
}
