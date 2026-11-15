//! Versioned JSON snapshots written by atomic replacement on macOS, Android and Linux.
//! Use for app preferences and restoration, not secrets. File I/O is synchronous;
//! save small snapshots on explicit changes or move large writes to a worker.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    error::Error,
    fmt, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Schema { expected: u32, found: u32 },
    TooLarge { limit: usize },
}
impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "snapshot I/O: {e}"),
            Self::Json(e) => write!(f, "snapshot JSON: {e}"),
            Self::Schema { expected, found } => write!(
                f,
                "snapshot schema {found} differs from expected {expected}"
            ),
            Self::TooLarge { limit } => write!(f, "snapshot exceeds {limit} bytes"),
        }
    }
}
impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}
impl From<std::io::Error> for StoreError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope<T> {
    schema: u32,
    data: T,
}

#[derive(Debug, Clone)]
pub struct JsonStore {
    path: PathBuf,
    schema: u32,
    max_bytes: usize,
}
impl JsonStore {
    pub fn new(path: impl Into<PathBuf>, schema: u32) -> Self {
        Self {
            path: path.into(),
            schema,
            max_bytes: 1024 * 1024,
        }
    }
    pub fn with_max_bytes(mut self, max: usize) -> Self {
        self.max_bytes = max;
        self
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// Missing files return `None`. Corrupt, incompatible and oversized files remain untouched.
    pub fn load<T: DeserializeOwned>(&self) -> Result<Option<T>, StoreError> {
        let file = match fs::File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let mut bytes = Vec::new();
        file.take(self.max_bytes.saturating_add(1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > self.max_bytes {
            return Err(StoreError::TooLarge {
                limit: self.max_bytes,
            });
        }
        // Inspect the schema before deserializing the application's possibly changed data type.
        let envelope: Envelope<serde_json::Value> = serde_json::from_slice(&bytes)?;
        if envelope.schema != self.schema {
            return Err(StoreError::Schema {
                expected: self.schema,
                found: envelope.schema,
            });
        }
        Ok(Some(serde_json::from_value(envelope.data)?))
    }
    /// Replace the last snapshot only after serialization, writing and sync succeed.
    /// Concurrent writers use independent temporary files; the last successful replacement wins.
    pub fn save<T: Serialize>(&self, data: &T) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec(&Envelope {
            schema: self.schema,
            data,
        })?;
        if bytes.len() > self.max_bytes {
            return Err(StoreError::TooLarge {
                limit: self.max_bytes,
            });
        }
        let parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(&self.path)
            .map_err(|e| StoreError::Io(e.error))?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    }
}

/// Resolve a desktop application data directory. The identifier must be a single
/// path component; spaces and punctuation are normalized to hyphens. A stable
/// reverse-domain app ID avoids collisions. Android runners use the activity path.
#[cfg(not(target_os = "android"))]
pub fn app_data_dir(app_id: &str) -> Option<PathBuf> {
    let id: String = app_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    if id.is_empty() || id == "." || id == ".." {
        return None;
    }
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
    base.filter(|p| p.is_absolute()).map(|p| p.join(id))
}
