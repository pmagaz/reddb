use std::fmt::{self, Display};
use thiserror::Error;
use uuid::Uuid;

pub type Result<T> = ::anyhow::Result<T, RedDbError>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Error)]
pub enum RedDbErrorKind {
    //STORAGE
    #[error("Data corrupted!")]
    DataCorruption,
    #[error("Data compacted corrupted!")]
    Compact,
    #[error("Could not compact storage")]
    Storagepersist,
    #[error("Could not flush data into storage")]
    FlushData,
    #[error("Could not flush data")]
    AppendData,
    #[error("Could not append data ")]
    StorageInit,
    #[error("Could not init storage")]
    StorageData,
    #[error("Could not read storage data")]
    ReadContent,
    #[error("Could not load storage content")]
    ContentLoad,
    #[error("Could not persist data into storage")]
    Datapersist,
    // uuids
    #[error("Could not find _id {_id}")]
    NotFound { _id: Uuid },
    #[error("Could not delete _id")]
    Deletekey,
    #[error("Could not unlock mutex")]
    Mutex,
    #[error("Database poisoned!")]
    Poisoned,
    #[error("data poisoned!")]
    PoisonedValue,
    // SERDE
    #[error("Could not deserialize data")]
    Deserialization,
    #[error("Could not serialize data")]
    Serialization,
}

#[derive(Debug, Error)]
pub struct RedDbError {
    err: RedDbErrorKind,
}

impl RedDbError {
    pub fn kind(&self) -> RedDbErrorKind {
        self.err
    }
}

impl From<RedDbErrorKind> for RedDbError {
    fn from(kind: RedDbErrorKind) -> RedDbError {
        RedDbError { err: kind }
    }
}

impl Display for RedDbError {
    fn fmt(&self, err: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.err, err)
    }
}
