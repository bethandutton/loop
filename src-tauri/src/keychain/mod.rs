use keyring::Entry;

const SERVICE_NAME: &str = "com.loop.app";

fn entry(key: &str) -> Result<Entry, String> {
    Entry::new(SERVICE_NAME, key).map_err(|e| format!("Keychain error: {}", e))
}

pub fn store_secret(key: &str, value: &str) -> Result<(), String> {
    let entry = entry(key)?;
    entry
        .set_password(value)
        .map_err(|e| format!("Failed to store secret: {}", e))
}

pub fn get_secret(key: &str) -> Result<Option<String>, String> {
    let entry = entry(key)?;
    match entry.get_password() {
        Ok(val) => Ok(Some(val)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read secret: {}", e)),
    }
}

pub fn delete_secret(key: &str) -> Result<(), String> {
    let entry = entry(key)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete secret: {}", e)),
    }
}
