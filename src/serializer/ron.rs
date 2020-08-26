use failure::{Error, ResultExt};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};
use crate::error::RedDbErrorKind;

#[derive(Debug)]
pub struct Ron {
  format: Serializers,
}

impl Default for Ron {
  fn default() -> Ron {
    Ron {
      format: Serializers::Ron(".ron".to_owned()),
    }
  }
}

pub type RonSerializer = Ron;

//#[cfg(feature = "ron_ser")]
impl<'a> Serializer<'a> for Ron {
  fn format(&self) -> &Serializers {
    &self.format
  }

  fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vec = ron::ser::to_string(data)
      .context(RedDbErrorKind::Serialization)?
      .into_bytes();
    vec.extend(b"\n");
    Ok(vec)
  }
  fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    Ok(ron::de::from_bytes::<T>(data)?)
  }
}
