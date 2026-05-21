use thiserror::Error;
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, RedDbError>;

#[derive(Debug, Error)]
pub enum RedDbError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization failed: {0}")]
    Serialize(String),

    #[error("deserialization failed: {0}")]
    Deserialize(String),

    #[error("document not found: {0}")]
    NotFound(Uuid),

    #[error("lock poisoned")]
    LockPoisoned,

    #[error("data corrupted")]
    DataCorrupted,

    #[error("persistence failed: {0}")]
    PersistFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays_uuid() {
        let id = Uuid::new_v4();
        let err = RedDbError::NotFound(id);
        assert_eq!(err.to_string(), format!("document not found: {}", id));
    }

    #[test]
    fn io_error_converts() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "no file");
        let db_err: RedDbError = io_err.into();
        assert!(matches!(db_err, RedDbError::Io(_)));
    }

    #[test]
    fn serialize_error_carries_message() {
        let err = RedDbError::Serialize("bad format".to_string());
        assert!(err.to_string().contains("bad format"));
    }

    #[test]
    fn deserialize_error_carries_message() {
        let err = RedDbError::Deserialize("unexpected byte".to_string());
        assert!(err.to_string().contains("unexpected byte"));
    }

    #[test]
    fn lock_poisoned_displays() {
        let err = RedDbError::LockPoisoned;
        assert_eq!(err.to_string(), "lock poisoned");
    }

    #[test]
    fn data_corrupted_displays() {
        let err = RedDbError::DataCorrupted;
        assert_eq!(err.to_string(), "data corrupted");
    }

    #[test]
    fn persist_failed_carries_message() {
        let err = RedDbError::PersistFailed("disk full".to_string());
        assert!(err.to_string().contains("disk full"));
    }
}
