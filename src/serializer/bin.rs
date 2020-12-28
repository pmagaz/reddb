use bincode::{deserialize_from, serialize};
use failure::Error;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};

#[derive(Debug)]
pub struct Bin {
  format: Serializers,
}

impl Default for Bin {
  fn default() -> Bin {
    Bin {
      format: Serializers::Bin(".bin".to_owned()),
    }
  }
}

pub type BinSerializer = Bin;

//#[cfg(feature = "bin_ser")]
impl<'a> Serializer<'a> for Bin {
  fn format(&self) -> &Serializers {
    &self.format
  }

  fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let mut vec = serialize(data)?;
    vec.extend(b"\n");
    Ok(vec)
  }
  fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
  where
    for<'de> T: Serialize + Deserialize<'de>,
  {
    let vec = deserialize_from(data)?;
    Ok(vec)
  }
}
