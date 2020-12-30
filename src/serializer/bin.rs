use std::default::Default;
use std::fmt::Debug;

use super::*;

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

#[cfg(feature = "bin_ser")]
impl<'a> Serializer<'a> for Bin {
    fn format(&self) -> &Serializers {
        &self.format
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: serde::Serialize + serde::Deserialize<'de>,
    {
        let mut vec = bincode::serialize(data)?;
        vec.extend(b"\n");
        Ok(vec)
    }
    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: serde::Serialize + serde::Deserialize<'de>,
    {
        let vec = bincode::deserialize_from(data)?;
        Ok(vec)
    }
}
