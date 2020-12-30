use std::default::Default;
use std::fmt::Debug;

use super::*;

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

#[cfg(feature = "yaml_ser")]
impl<'a> Serializer<'a> for Yaml {
    fn format(&self) -> &Serializers {
        &self.format
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: serde::Serialize + serde::Deserialize<'de>,
    {
        let mut vec = serde_yaml::to_vec(data)?;
        println!("{:?}4444444444", serde_yaml::to_string(data)?);

        //let mut vec = serde_yaml::to_string(data)?.into_bytes();
        vec.extend(b"\n");
        Ok(vec)
    }
    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: serde::Serialize + serde::Deserialize<'de>,
    {
        Ok(serde_yaml::from_reader(data).unwrap())
    }
}
