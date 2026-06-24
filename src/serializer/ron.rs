use super::{FormatId, Serializer};
use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default)]
pub struct Ron;

#[cfg(feature = "ron_ser")]
impl Serializer for Ron {
    fn format_id(&self) -> FormatId {
        FormatId::Ron
    }

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(::ron::ser::to_string(data)?.into_bytes())
    }

    fn deserialize<T>(&self, data: &[u8]) -> Result<T, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>,
    {
        Ok(::ron::de::from_bytes(data)?)
    }
}

#[cfg(test)]
#[cfg(feature = "ron_ser")]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct S {
        x: u32,
    }

    #[test]
    fn round_trip() {
        let s = S { x: 7 };
        let ser = Ron.serialize(&s).unwrap();
        let de: S = Ron.deserialize(&ser).unwrap();
        assert_eq!(de, s);
    }

    #[test]
    fn no_trailing_newline() {
        let s = S { x: 1 };
        let bytes = Ron.serialize(&s).unwrap();
        assert_ne!(bytes.last().copied(), Some(b'\n'));
    }

    #[test]
    fn format_id_is_ron() {
        assert_eq!(Ron.format_id(), FormatId::Ron);
    }
}
