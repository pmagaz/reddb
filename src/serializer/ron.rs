use failure::{Error, ResultExt};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};
use crate::error::RdStoreErrorKind;

#[derive(Debug)]
pub struct Ron {
  format: Serializers,
}

impl Default for Ron {
  fn default() -> Ron {
    Ron {
      format: Serializers::Ron(".ron.db".to_owned()),
    }
  }
}

pub type RonSerializer = Ron;

//#[cfg(feature = "ron_ser")]
impl<'a> Serializer<'a> for Ron {
  fn format(&self) -> &Serializers {
    &self.format
  }

  fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vec = ron::ser::to_string(value)
      .context(RdStoreErrorKind::Serialization)?
      .into_bytes();
    vec.extend("\n".as_bytes());
    Ok(vec)
  }
  fn deserialize<T>(&self, value: &Vec<u8>) -> Result<T, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    Ok(ron::de::from_bytes::<T>(value)?) //.context(RdStoreErrorKind::Deserialization)?)
  }
}
