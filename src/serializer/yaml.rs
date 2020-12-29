use failure::Error;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::Debug;

use super::{Serializer, Serializers};

#[derive(Debug)]
pub struct Yaml {
    format: Serializers,
}

impl Default for Yaml {
    fn default() -> Yaml {
        Yaml {
            format: Serializers::Yaml(".yaml".to_owned()),
        }
    }
}

pub type YamlSerializer = Yaml;

//ca#[cfg(feature = "yaml_ser")]
impl<'a> Serializer<'a> for Yaml {
    fn format(&self) -> &Serializers {
        &self.format
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let mut vec = serde_yaml::to_vec(data).unwrap();
        vec.extend(b"\n");
        Ok(vec)
    }
    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(serde_yaml::from_slice::<T>(data).unwrap())
    }
}
