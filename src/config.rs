use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DbConfig {
    pub name: String,
    pub dir: PathBuf,
    /// Trigger compaction when file_size >= live_data_size * ratio.
    /// Default: 2.0 — compact when file is 2× larger than live data.
    pub compaction_ratio: f64,
}

impl DbConfig {
    pub fn new(name: impl Into<String>) -> Self {
        DbConfig {
            name: name.into(),
            dir: PathBuf::from("."),
            compaction_ratio: 2.0,
        }
    }

    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.dir = dir.into();
        self
    }

    pub fn compaction_ratio(mut self, ratio: f64) -> Self {
        self.compaction_ratio = ratio;
        self
    }

    pub fn file_stem(&self) -> PathBuf {
        self.dir.join(&self.name)
    }
}

impl Default for DbConfig {
    fn default() -> Self {
        DbConfig::new("reddb")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_dir_is_current() {
        let cfg = DbConfig::new("mydb");
        assert_eq!(cfg.dir, PathBuf::from("."));
    }

    #[test]
    fn default_compaction_ratio() {
        let cfg = DbConfig::new("mydb");
        assert_eq!(cfg.compaction_ratio, 2.0);
    }

    #[test]
    fn builder_overrides_dir() {
        let cfg = DbConfig::new("mydb").dir("/tmp");
        assert_eq!(cfg.dir, PathBuf::from("/tmp"));
    }

    #[test]
    fn builder_overrides_compaction_ratio() {
        let cfg = DbConfig::new("mydb").compaction_ratio(3.5);
        assert_eq!(cfg.compaction_ratio, 3.5);
    }

    #[test]
    fn file_stem_joins_dir_and_name() {
        let cfg = DbConfig::new("users").dir("/data");
        assert_eq!(cfg.file_stem(), PathBuf::from("/data/users"));
    }

    #[test]
    fn default_impl_uses_reddb_name() {
        let cfg = DbConfig::default();
        assert_eq!(cfg.name, "reddb");
    }
}
