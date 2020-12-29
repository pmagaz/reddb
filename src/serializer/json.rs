use failure::Error;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};

#[derive(Debug)]
pub struct Json {
    format: Serializers,
}

impl Default for Json {
    fn default() -> Json {
        Json {
            format: Serializers::Json(".json".to_owned()),
        }
    }
}

pub type JsonSerializer = Json;

//#[cfg(feature = "json_ser")]
impl<'a> Serializer<'a> for Json {
    fn format(&self) -> &Serializers {
        &self.format
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let mut vec = serde_json::to_vec(data)?;
        vec.extend(b"\n");
        Ok(vec)
    }
    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let vec = serde_json::from_slice::<T>(data)?;
        Ok(vec)
    }
}
