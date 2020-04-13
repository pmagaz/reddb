use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};
use uuid::Uuid;

pub type Result<T> = ::std::result::Result<T, RdStoreError>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum RdStoreErrorKind {
  //STORAGE
  #[fail(display = "Storage corrupted!")]
  DataCorruption,
  #[fail(display = "Storage corrupted!")]
  Compact,
  #[fail(display = "Could not compact storage")]
  StorageSave,
  #[fail(display = "Could not flush data into storage")]
  FlushData,
  #[fail(display = "Could not append data to storage")]
  AppendData,
  #[fail(display = "Could not open storage")]
  StorageOpen,
  #[fail(display = "Could not read storage")]
  ReadContent,
  #[fail(display = "Could not load storage content")]
  ContentLoad,
  #[fail(display = "Could not save data into storage")]
  DataSave,
  //STORE
  #[fail(display = "Could not insert value")]
  InsertValue,
  #[fail(display = "Could not update value")]
  UpdateValue,
  #[fail(display = "Could not delete value")]
  DeleteValue,
  // KEYS
  #[fail(display = "Could not find key {}", key)]
  NotFound { key: Uuid },
  #[fail(display = "Could not delete key")]
  Deletekey,
  #[fail(display = "Could not unlock mutex")]
  Mutex,
  #[fail(display = "Database poisoned!")]
  Poisoned,
  #[fail(display = "Value poisoned!")]
  PoisonedValue,
  // SERDE
  #[fail(display = "Could not deserialize value")]
  Deserialization,
  #[fail(display = "Could not serialize value")]
  Serialization,
}

#[derive(Debug)]
pub struct RdStoreError {
  inner: Context<RdStoreErrorKind>,
}

impl RdStoreError {
  pub fn kind(&self) -> RdStoreErrorKind {
    *self.inner.get_context()
  }
}

impl From<RdStoreErrorKind> for RdStoreError {
  fn from(kind: RdStoreErrorKind) -> RdStoreError {
    RdStoreError {
      inner: Context::new(kind),
    }
  }
}

impl From<Context<RdStoreErrorKind>> for RdStoreError {
  fn from(inner: Context<RdStoreErrorKind>) -> RdStoreError {
    RdStoreError { inner }
  }
}

impl Fail for RdStoreError {
  fn cause(&self) -> Option<&dyn Fail> {
    self.inner.cause()
  }

  fn backtrace(&self) -> Option<&Backtrace> {
    self.inner.backtrace()
  }
}

impl Display for RdStoreError {
  fn fmt(&self, err: &mut fmt::Formatter<'_>) -> fmt::Result {
    Display::fmt(&self.inner, err)
  }
}
