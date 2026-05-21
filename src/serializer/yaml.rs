use super::{FormatId, Serializer};
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct Yaml;

#[cfg(feature = "yaml_ser")]
impl Serializer for Yaml {
    fn format_id(&self) -> FormatId {
        FormatId::Yaml
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let mut bytes = serde_yaml::to_string(data)?.into_bytes();
        // serde_yaml appends a trailing newline; strip it so record framing
        // is handled exclusively by the storage layer.
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
        }
        Ok(bytes)
    }

    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(serde_yaml::from_slice(data)?)
    }
}

#[cfg(test)]
#[cfg(feature = "yaml_ser")]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct S {
        x: u32,
    }

    #[test]
    fn round_trip() {
        let s = S { x: 99 };
        let ser = Yaml.serialize(&s).unwrap();
        let de: S = Yaml.deserialize(&ser).unwrap();
        assert_eq!(de, s);
    }

    #[test]
    fn no_trailing_newline() {
        let s = S { x: 1 };
        let bytes = Yaml.serialize(&s).unwrap();
        assert_ne!(bytes.last().copied(), Some(b'\n'));
    }

    #[test]
    fn format_id_is_yaml() {
        assert_eq!(Yaml.format_id(), FormatId::Yaml);
    }
}
