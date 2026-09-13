//! Disk-backed [`LockStore`].

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{LockError, LockStore, validate_resource};

const MAGIC: &[u8; 4] = b"SLCK";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 13; // magic(4) + version(1) + expires_ms(8)

/// Configuration for [`FilesystemLockStore`].
#[derive(Clone, Debug)]
pub struct FilesystemLockStoreConfig {
    directory: PathBuf,
    prefix: String,
}

impl FilesystemLockStoreConfig {
    /// Root directory that will contain the prefix subdirectory.
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            prefix: "serenade".to_owned(),
        }
    }

    /// Sets the single path-segment prefix (default `serenade`).
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Configured root directory.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Configured prefix segment.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

/// Disk-backed [`LockStore`] (one file per resource).
#[derive(Debug)]
pub struct FilesystemLockStore {
    root: PathBuf,
}

impl FilesystemLockStore {
    /// Opens (creating) `{directory}/{prefix}` for lock files.
    ///
    /// # Errors
    ///
    /// Returns [`LockError::Store`] when the prefix is invalid or directories cannot be created.
    pub fn open(config: FilesystemLockStoreConfig) -> Result<Self, LockError> {
        let FilesystemLockStoreConfig { directory, prefix } = config;
        validate_prefix(&prefix)?;
        let root = directory.join(&prefix);
        fs::create_dir_all(&root).map_err(|error| LockError::Store {
            message: format!("create {}: {error}", root.display()),
        })?;
        Ok(Self { root })
    }

    fn path_for(&self, resource: &str) -> PathBuf {
        self.root.join(format!("{}.lock", hex_key(resource)))
    }

    fn read_entry(&self, resource: &str) -> Result<Option<(u64, String)>, LockError> {
        let path = self.path_for(resource);
        match fs::read(&path) {
            Ok(bytes) => {
                let Some((expires, token)) = parse_record(&bytes) else {
                    let _ = fs::remove_file(&path);
                    return Ok(None);
                };
                if is_expired_unix_ms(expires) {
                    let _ = fs::remove_file(&path);
                    return Ok(None);
                }
                Ok(Some((expires, token)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(LockError::Store {
                message: format!("read {}: {error}", path.display()),
            }),
        }
    }

    fn write_entry(
        &self,
        resource: &str,
        token: &str,
        ttl: Option<Duration>,
    ) -> Result<(), LockError> {
        let path = self.path_for(resource);
        let expires = unix_ms_from_ttl(ttl);
        let record = encode_record(expires, token.as_bytes());
        atomic_write(&path, &record)
    }
}

impl LockStore for FilesystemLockStore {
    fn save(&self, resource: &str, token: &str, ttl: Option<Duration>) -> Result<(), LockError> {
        validate_resource(resource)?;
        if let Some((_expires, held)) = self.read_entry(resource)? {
            if held != token {
                return Err(LockError::Conflict {
                    key: resource.to_owned(),
                });
            }
        }
        self.write_entry(resource, token, ttl)
    }

    fn delete(&self, resource: &str, token: &str) -> Result<(), LockError> {
        validate_resource(resource)?;
        let path = self.path_for(resource);
        match self.read_entry(resource)? {
            Some((_expires, held)) if held == token => match fs::remove_file(&path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(LockError::Store {
                    message: format!("delete {}: {error}", path.display()),
                }),
            },
            Some(_) | None => Ok(()),
        }
    }

    fn exists(&self, resource: &str, token: &str) -> Result<bool, LockError> {
        validate_resource(resource)?;
        Ok(matches!(self.read_entry(resource)?, Some((_expires, held)) if held == token))
    }

    fn put_off_expiration(
        &self,
        resource: &str,
        token: &str,
        ttl: Option<Duration>,
    ) -> Result<(), LockError> {
        validate_resource(resource)?;
        match self.read_entry(resource)? {
            Some((_expires, held)) if held == token => self.write_entry(resource, token, ttl),
            Some(_) | None => Err(LockError::NotHeld {
                key: resource.to_owned(),
            }),
        }
    }
}

fn validate_prefix(prefix: &str) -> Result<(), LockError> {
    if prefix.is_empty()
        || prefix.contains('/')
        || prefix.contains('\\')
        || prefix == "."
        || prefix == ".."
    {
        return Err(LockError::Store {
            message: "filesystem lock prefix must be a single non-empty path segment".to_owned(),
        });
    }
    Ok(())
}

fn hex_key(key: &str) -> String {
    key.as_bytes()
        .iter()
        .fold(String::with_capacity(key.len() * 2), |mut out, byte| {
            out.push(char::from_digit(u32::from(*byte >> 4), 16).unwrap_or('0'));
            out.push(char::from_digit(u32::from(*byte & 0x0f), 16).unwrap_or('0'));
            out
        })
}

fn encode_record(expires_unix_ms: u64, token: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + token.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&expires_unix_ms.to_le_bytes());
    out.extend_from_slice(token);
    out
}

fn parse_record(bytes: &[u8]) -> Option<(u64, String)> {
    if bytes.len() < HEADER_LEN || &bytes[..4] != MAGIC || bytes[4] != VERSION {
        return None;
    }
    let mut expiry_bytes = [0_u8; 8];
    expiry_bytes.copy_from_slice(&bytes[5..13]);
    let expires_unix_ms = u64::from_le_bytes(expiry_bytes);
    let token = String::from_utf8(bytes[HEADER_LEN..].to_vec()).ok()?;
    Some((expires_unix_ms, token))
}

fn is_expired_unix_ms(expires_unix_ms: u64) -> bool {
    if expires_unix_ms == 0 {
        return false;
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(u64::MAX, |now| {
            u64::try_from(now.as_millis()).unwrap_or(u64::MAX)
        });
    now_ms >= expires_unix_ms
}

fn unix_ms_from_ttl(ttl: Option<Duration>) -> u64 {
    let Some(ttl) = ttl else {
        return 0;
    };
    let now = SystemTime::now();
    let now_ms = now.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(0)
    });
    let expires = now
        .checked_add(ttl)
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        });
    if expires <= now_ms {
        now_ms.saturating_add(1)
    } else {
        expires
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), LockError> {
    let tmp = path.with_extension("lock.tmp");
    {
        let mut file = fs::File::create(&tmp).map_err(|error| LockError::Store {
            message: format!("create {}: {error}", tmp.display()),
        })?;
        file.write_all(bytes).map_err(|error| LockError::Store {
            message: format!("write {}: {error}", tmp.display()),
        })?;
        let _ = file.sync_all();
    }
    fs::rename(&tmp, path).map_err(|error| LockError::Store {
        message: format!("rename {} -> {}: {error}", tmp.display(), path.display()),
    })
}
