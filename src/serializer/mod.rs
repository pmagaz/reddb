use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};

mod bin;
mod json;
mod ron;
mod yaml;

#[cfg(feature = "bin_ser")]
pub use self::bin::Bin;
#[cfg(feature = "json_ser")]
pub use self::json::Json;
#[cfg(feature = "ron_ser")]
pub use self::ron::Ron;
#[cfg(feature = "yaml_ser")]
pub use self::yaml::Yaml;

/// Identifies the serialization format and its file extension.
/// The `u8` discriminant is stored in the 32-byte file header (byte 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FormatId {
    Json = 0,
    Ron  = 1,
    Yaml = 2,
    Bin  = 3,
}

impl FormatId {
    pub fn extension(self) -> &'static str {
        match self {
            FormatId::Bin  => ".bin",
            FormatId::Json => ".json",
            FormatId::Yaml => ".yaml",
            FormatId::Ron  => ".ron",
        }
    }
}

/// Pluggable serialization backend.
/// Implementations must not append any delimiter bytes (e.g. `\n`);
/// record framing is handled by the storage layer.
pub trait Serializer: Default + Send + Sync {
    fn format_id(&self) -> FormatId;

    fn serialize<T>(&self, val: &T) -> Result<Vec<u8>, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>;

    fn deserialize<T>(&self, val: &[u8]) -> Result<T, Error>
    where
        for<'de> T: Serialize + Deserialize<'de>;
}

// Keep backward-compatible re-export used by FileStorage to get the extension
pub use FormatId as Serializers;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_id_extensions() {
        assert_eq!(FormatId::Bin.extension(),  ".bin");
        assert_eq!(FormatId::Json.extension(), ".json");
        assert_eq!(FormatId::Yaml.extension(), ".yaml");
        assert_eq!(FormatId::Ron.extension(),  ".ron");
    }

    #[test]
    fn format_id_copy_and_eq() {
        let a = FormatId::Json;
        let b = a;  // Copy
        assert_eq!(a, b);
        assert_ne!(FormatId::Json, FormatId::Bin);
    }
}
