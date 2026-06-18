use crate::error::{AppError, AppResult};
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "hehel-zip";
const USER: &str = "hcom_session";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSession {
    pub session_token: String,
    pub hcom_api_url: String,
    pub heron_auth_url: String,
    pub expires_at: Option<String>,
}

pub fn write_session(session: &AuthSession) -> AppResult<()> {
    let entry = Entry::new(SERVICE, USER)
        .map_err(|e| AppError::Auth(format!("keyring: {e}")))?;
    let json = serde_json::to_string(session)
        .map_err(|e| AppError::Auth(format!("serialize: {e}")))?;
    entry
        .set_password(&json)
        .map_err(|e| AppError::Auth(format!("keyring write: {e}")))?;
    Ok(())
}

pub fn read_session() -> AppResult<Option<AuthSession>> {
    let entry = Entry::new(SERVICE, USER)
        .map_err(|e| AppError::Auth(format!("keyring: {e}")))?;
    match entry.get_password() {
        Ok(json) => {
            let session: AuthSession = serde_json::from_str(&json)
                .map_err(|e| AppError::Auth(format!("parse session: {e}")))?;
            Ok(Some(session))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Auth(format!("keyring read: {e}"))),
    }
}

pub fn get_session_token() -> AppResult<Option<String>> {
    Ok(read_session()?.map(|s| s.session_token))
}

pub fn has_session() -> AppResult<bool> {
    Ok(read_session()?.is_some())
}

pub fn clear_session() -> AppResult<()> {
    let entry = Entry::new(SERVICE, USER)
        .map_err(|e| AppError::Auth(format!("keyring: {e}")))?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Auth(format!("keyring delete: {e}"))),
    }
}
