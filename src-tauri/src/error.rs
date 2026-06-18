use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Ошибка архива: {0}")]
    Archive(String),
    #[error("Ошибка базы данных: {0}")]
    Database(String),
    #[error("Ошибка синхронизации: {0}")]
    Sync(String),
    #[error("Ошибка валидации: {0}")]
    Validation(String),
    #[error("Ошибка аутентификации: {0}")]
    Auth(String),
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("Zip Slip: {entry} — {reason}")]
    ArchiveZipSlip { entry: String, reason: String },
    #[error("Запись не найдена: {0}")]
    ArchiveEntryNotFound(String),
    #[error("Зарезервированное имя Windows: {0}")]
    ArchiveReservedName(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
