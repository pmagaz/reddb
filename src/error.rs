use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};

pub type Result<T> = ::std::result::Result<T, RdStoreError>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum RdStoreErrorKind {
  #[fail(display = "Could not open file")]
  File,
  #[fail(display = "Could not serialize value")]
  Serialize,
  #[fail(display = "Could not deseralize value")]
  Deserialize,
  #[fail(display = "Database poisoned!")]
  Poisoned,
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
