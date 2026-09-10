use std::path::PathBuf;

pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub cache_dir: PathBuf,
    pub cache_ttl_secs: u64,
}

impl Config {
    /// The SQLite file behind `database_url`, if it has one. Handles the
    /// `sqlite://`, `sqlite:` and bare-path forms; `:memory:` has no file.
    pub fn db_path(&self) -> PathBuf {
        let raw = self
            .database_url
            .strip_prefix("sqlite://")
            .or_else(|| self.database_url.strip_prefix("sqlite:"))
            .unwrap_or(&self.database_url);
        let raw = raw.split('?').next().unwrap_or(raw);
        if raw.is_empty() || raw.contains(":memory:") {
            PathBuf::new()
        } else {
            PathBuf::from(raw)
        }
    }

    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://gh-projects.db".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3001),
            cache_dir: std::env::var("CACHE_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(".cache")),
            cache_ttl_secs: std::env::var("CACHE_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
        }
    }
}
