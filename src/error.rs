use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};
use uuid::Uuid;

pub type Result<T> = ::std::result::Result<T, RedDbError>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum RedDbErrorKind {
  //STORAGE
  #[fail(display = "Data corrupted!")]
  DataCorruption,
  #[fail(display = "Data compacted corrupted!")]
  Compact,
  #[fail(display = "Could not compact storage")]
  Storagepersist,
  #[fail(display = "Could not flush data into storage")]
  FlushData,
  #[fail(display = "Could not flush data")]
  AppendData,
  #[fail(display = "Could not append data ")]
  StorageOpen,
  #[fail(display = "Could not read storageeee")]
  ReadContent,
  #[fail(display = "Could not load storage content")]
  ContentLoad,
  #[fail(display = "Could not persist data into storage")]
  Datapersist,
  // uuids
  #[fail(display = "Could not find uuid {}", uuid)]
  NotFound { uuid: Uuid },
  #[fail(display = "Could not delete uuid")]
  Deletekey,
  #[fail(display = "Could not unlock mutex")]
  Mutex,
  #[fail(display = "Database poisoned!")]
  Poisoned,
  #[fail(display = "data poisoned!")]
  PoisonedValue,
  // SERDE
  #[fail(display = "Could not deserialize data")]
  Deserialization,
  #[fail(display = "Could not serialize data")]
  Serialization,
}

#[derive(Debug)]
pub struct RedDbError {
  err: Context<RedDbErrorKind>,
}

impl RedDbError {
  pub fn kind(&self) -> RedDbErrorKind {
    *self.err.get_context()
  }
}

impl From<RedDbErrorKind> for RedDbError {
  fn from(kind: RedDbErrorKind) -> RedDbError {
    RedDbError {
      err: Context::new(kind),
    }
  }
}

impl From<Context<RedDbErrorKind>> for RedDbError {
  fn from(err: Context<RedDbErrorKind>) -> RedDbError {
    RedDbError { err }
  }
}

impl Fail for RedDbError {
  fn cause(&self) -> Option<&dyn Fail> {
    self.err.cause()
  }

  fn backtrace(&self) -> Option<&Backtrace> {
    self.err.backtrace()
  }
}

impl Display for RedDbError {
  fn fmt(&self, err: &mut fmt::Formatter<'_>) -> fmt::Result {
    Display::fmt(&self.err, err)
  }
}
