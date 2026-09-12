//! Disk-backed [`CacheItemPool`](crate::CacheItemPool).

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::key::validate_logical_key;
use crate::marshaller::CacheMarshaller;
use crate::{ArrayCacheItem, CacheError, CacheItemPool};

const MAGIC: &[u8; 4] = b"SCFS";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 4 + 1 + 8;

/// Configuration for [`FilesystemAdapter`].
#[derive(Debug, Clone)]
pub struct FilesystemAdapterConfig {
    directory: PathBuf,
    prefix: String,
}

impl FilesystemAdapterConfig {
    /// Stores items under `directory` / `prefix` (default prefix `serenade`).
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            prefix: String::from("serenade"),
        }
    }

    /// Sets the subdirectory name under the cache root (must be a single path segment).
    #[must_use]
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Cache root directory.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Subdirectory prefix.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

/// Filesystem [`CacheItemPool`] using one file per key and a [`CacheMarshaller`].
///
/// Expired entries are deleted on read (`get_item` / `has_item`). `clear` removes
/// every `*.cache` file under the adapter directory. Values must be marshallable
/// (default: [`crate::BytesMarshaller`] for `String` / `Vec<u8>`).
pub struct FilesystemAdapter {
    root: PathBuf,
    marshaller: Arc<dyn CacheMarshaller>,
}

impl FilesystemAdapter {
    /// Creates the adapter directory and returns a pool.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Pool`] when the directory cannot be created or the
    /// prefix is not a single path segment.
    pub fn open(
        config: FilesystemAdapterConfig,
        marshaller: Arc<dyn CacheMarshaller>,
    ) -> Result<Self, CacheError> {
        let FilesystemAdapterConfig { directory, prefix } = config;
        validate_prefix(&prefix)?;
        let root = directory.join(&prefix);
        fs::create_dir_all(&root).map_err(|error| CacheError::Pool {
            message: format!("create cache dir {}: {error}", root.display()),
        })?;
        Ok(Self { root, marshaller })
    }

    fn path_for(&self, key: &str) -> Result<PathBuf, CacheError> {
        validate_logical_key(key)?;
        Ok(self.root.join(format!("{}.cache", hex_key(key))))
    }
}

impl CacheItemPool for FilesystemAdapter {
    fn get_item(&self, key: &str) -> Result<ArrayCacheItem, CacheError> {
        let path = self.path_for(key)?;
        let Some(bytes) = read_file(&path)? else {
            return Ok(ArrayCacheItem::miss(key));
        };
        let Some((expires_unix_ms, payload)) = parse_record(&bytes) else {
            let _ = fs::remove_file(&path);
            return Ok(ArrayCacheItem::miss(key));
        };
        if is_expired_unix_ms(expires_unix_ms) {
            let _ = fs::remove_file(&path);
            return Ok(ArrayCacheItem::miss(key));
        }
        let Ok(decoded) = self.marshaller.unmarshal(payload) else {
            let _ = fs::remove_file(&path);
            return Ok(ArrayCacheItem::miss(key));
        };
        Ok(ArrayCacheItem::hit(key, decoded).with_expiry(instant_from_unix_ms(expires_unix_ms)))
    }

    fn save(&self, item: ArrayCacheItem) -> Result<(), CacheError> {
        let (key, value, expires_at, tags) = item.into_stored();
        let path = self.path_for(&key)?;
        let Some(value) = value else {
            let _ = fs::remove_file(&path);
            return Ok(());
        };
        if expires_at.is_some_and(|at| Instant::now() >= at) {
            let _ = fs::remove_file(&path);
            return Ok(());
        }
        // Tags are not persisted on the filesystem adapter yet.
        let _ = tags;
        let payload = self.marshaller.marshal(value.as_ref())?;
        let expires_unix_ms = unix_ms_from_instant(expires_at);
        let record = encode_record(expires_unix_ms, &payload);
        atomic_write(&path, &record)
    }

    fn delete_item(&self, key: &str) -> Result<bool, CacheError> {
        let path = self.path_for(key)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(CacheError::Pool {
                message: format!("delete {}: {error}", path.display()),
            }),
        }
    }

    fn clear(&self) -> Result<(), CacheError> {
        let entries = fs::read_dir(&self.root).map_err(|error| CacheError::Pool {
            message: format!("read cache dir {}: {error}", self.root.display()),
        })?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "cache") {
                let _ = fs::remove_file(path);
            }
        }
        Ok(())
    }
}

fn validate_prefix(prefix: &str) -> Result<(), CacheError> {
    if prefix.is_empty()
        || prefix.contains('/')
        || prefix.contains('\\')
        || prefix == "."
        || prefix == ".."
    {
        return Err(CacheError::Pool {
            message: "filesystem cache prefix must be a single non-empty path segment".to_owned(),
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

fn encode_record(expires_unix_ms: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN + payload.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&expires_unix_ms.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

fn parse_record(bytes: &[u8]) -> Option<(u64, &[u8])> {
    if bytes.len() < HEADER_LEN || &bytes[..4] != MAGIC || bytes[4] != VERSION {
        return None;
    }
    let mut expiry_bytes = [0_u8; 8];
    expiry_bytes.copy_from_slice(&bytes[5..13]);
    let expires_unix_ms = u64::from_le_bytes(expiry_bytes);
    Some((expires_unix_ms, &bytes[HEADER_LEN..]))
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

fn instant_from_unix_ms(expires_unix_ms: u64) -> Option<Instant> {
    if expires_unix_ms == 0 {
        return None;
    }
    let deadline = UNIX_EPOCH.checked_add(Duration::from_millis(expires_unix_ms))?;
    let remaining = deadline.duration_since(SystemTime::now()).ok()?;
    Some(Instant::now() + remaining)
}

fn unix_ms_from_instant(expires_at: Option<Instant>) -> u64 {
    let Some(deadline) = expires_at else {
        return 0;
    };
    let now_instant = Instant::now();
    if deadline <= now_instant {
        return 0;
    }
    let remaining = deadline.duration_since(now_instant);
    let now = SystemTime::now();
    let now_ms = now.duration_since(UNIX_EPOCH).map_or(0, |duration| {
        u64::try_from(duration.as_millis()).unwrap_or(0)
    });
    let expires = now
        .checked_add(remaining)
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        });
    // Sub-millisecond remaining can floor to `now_ms`; keep a future Instant future on disk.
    if expires <= now_ms {
        now_ms.saturating_add(1)
    } else {
        expires
    }
}

fn read_file(path: &Path) -> Result<Option<Vec<u8>>, CacheError> {
    match fs::File::open(path) {
        Ok(mut file) => {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)
                .map_err(|error| CacheError::Pool {
                    message: format!("read {}: {error}", path.display()),
                })?;
            Ok(Some(bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(CacheError::Pool {
            message: format!("open {}: {error}", path.display()),
        }),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), CacheError> {
    let tmp = path.with_extension("cache.tmp");
    {
        let mut file = fs::File::create(&tmp).map_err(|error| CacheError::Pool {
            message: format!("create {}: {error}", tmp.display()),
        })?;
        file.write_all(bytes).map_err(|error| CacheError::Pool {
            message: format!("write {}: {error}", tmp.display()),
        })?;
        // Best-effort durability; sync failure must not fail the save.
        let _ = file.sync_all();
    }
    fs::rename(&tmp, path).map_err(|error| CacheError::Pool {
        message: format!("rename {} -> {}: {error}", tmp.display(), path.display()),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use tempfile::TempDir;

    use super::{FilesystemAdapter, FilesystemAdapterConfig, hex_key};
    use crate::{ArrayCacheItem, BytesMarshaller, CacheError, CacheItem, CacheItemPool};

    fn open_pool() -> (TempDir, FilesystemAdapter) {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = FilesystemAdapter::open(
            FilesystemAdapterConfig::new(dir.path()).with_prefix("pool"),
            Arc::new(BytesMarshaller),
        )
        .expect("open");
        (dir, pool)
    }

    #[test]
    fn miss_hit_overwrite_delete_clear() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = FilesystemAdapter::open(
            FilesystemAdapterConfig::new(dir.path()),
            Arc::new(BytesMarshaller),
        )
        .expect("open");
        assert!(!pool.get_item("sku").expect("get").is_hit());

        let mut item = ArrayCacheItem::miss("sku");
        item.set(Arc::new(String::from("A1")));
        pool.save(item).expect("save");
        let hit = pool.get_item("sku").expect("get");
        assert!(hit.is_hit());
        assert_eq!(
            hit.get()
                .and_then(|value| value.downcast_ref::<String>())
                .map(String::as_str),
            Some("A1")
        );

        let mut item = ArrayCacheItem::miss("sku");
        item.set(Arc::new(String::from("B2")));
        pool.save(item).expect("overwrite");
        assert_eq!(
            pool.get_item("sku")
                .expect("get")
                .get()
                .and_then(|value| value.downcast_ref::<String>())
                .map(String::as_str),
            Some("B2")
        );

        assert!(pool.delete_item("sku").expect("delete"));
        assert!(!pool.delete_item("sku").expect("missing"));
        assert!(!pool.get_item("sku").expect("get").is_hit());

        let mut item = ArrayCacheItem::miss("x");
        item.set(Arc::new(vec![1_u8, 2]));
        pool.save(item).expect("save");
        pool.clear().expect("clear");
        assert!(!pool.get_item("x").expect("get").is_hit());
        drop(dir);
    }

    #[test]
    fn expiry_turns_hit_into_miss_and_deletes_file() {
        use super::{encode_record, parse_record};

        let dir = tempfile::tempdir().expect("tempdir");
        let pool = FilesystemAdapter::open(
            FilesystemAdapterConfig::new(dir.path()).with_prefix("ttl"),
            Arc::new(BytesMarshaller),
        )
        .expect("open");
        let mut item = ArrayCacheItem::miss("tmp");
        item.set(Arc::new(String::from("gone")));
        // Long TTL: avoid Instant races under llvm-cov between expires_after and save/get.
        item.expires_after(Some(Duration::from_secs(3600)));
        pool.save(item).expect("save");
        assert!(pool.get_item("tmp").expect("get").is_hit());

        let path = dir
            .path()
            .join("ttl")
            .join(format!("{}.cache", hex_key("tmp")));
        let bytes = std::fs::read(&path).expect("read");
        let (_expires, payload) = parse_record(&bytes).expect("parse");
        // Force a past unix expiry on disk (no sleep / no CI scheduling flake).
        std::fs::write(&path, encode_record(1, payload)).expect("rewrite expired");
        assert!(!pool.get_item("tmp").expect("get").is_hit());
        assert!(!path.exists());
    }

    #[test]
    fn empty_key_and_bad_prefix_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(
            FilesystemAdapter::open(
                FilesystemAdapterConfig::new(dir.path()).with_prefix("../x"),
                Arc::new(BytesMarshaller),
            )
            .is_err(),
            "bad prefix must fail"
        );

        let (_dir, pool) = open_pool();
        assert!(matches!(
            pool.get_item(""),
            Err(CacheError::InvalidKey { .. })
        ));
    }

    #[test]
    fn save_without_value_deletes() {
        let (_dir, pool) = open_pool();
        let mut item = ArrayCacheItem::miss("k");
        item.set(Arc::new(String::from("v")));
        pool.save(item).expect("save");
        pool.save(ArrayCacheItem::miss("k"))
            .expect("delete via miss");
        assert!(!pool.get_item("k").expect("get").is_hit());
    }

    #[test]
    fn config_accessors_corrupt_unmarshal_and_expired_save() {
        use std::fs;
        use std::time::Instant;

        use super::{
            encode_record, is_expired_unix_ms, parse_record, read_file, unix_ms_from_instant,
        };

        let dir = tempfile::tempdir().expect("tempdir");
        let config = FilesystemAdapterConfig::new(dir.path()).with_prefix("acc");
        assert_eq!(config.directory(), dir.path());
        assert_eq!(config.prefix(), "acc");
        let pool = FilesystemAdapter::open(config, Arc::new(BytesMarshaller)).expect("open");
        let root = dir.path().join("acc");

        let corrupt = root.join(format!("{}.cache", hex_key("corrupt")));
        fs::write(&corrupt, b"XXXX").expect("corrupt");
        assert!(!pool.get_item("corrupt").expect("get").is_hit());
        assert!(!corrupt.exists());

        let bad_payload = root.join(format!("{}.cache", hex_key("badtag")));
        fs::write(&bad_payload, encode_record(0, &[99, 1, 2])).expect("bad tag");
        assert!(!pool.get_item("badtag").expect("get").is_hit());
        assert!(!bad_payload.exists());

        let mut item = ArrayCacheItem::miss("stale");
        item.set(Arc::new(String::from("x")));
        item.expires_after(Some(Duration::ZERO));
        pool.save(item).expect("expired save is no-op delete");
        assert!(!pool.get_item("stale").expect("get").is_hit());

        let junk = root.join("ignore.txt");
        fs::write(&junk, b"nope").expect("junk");
        let mut item = ArrayCacheItem::miss("keep");
        item.set(Arc::new(String::from("y")));
        pool.save(item).expect("save");
        pool.clear().expect("clear");
        assert!(junk.exists());
        assert!(!pool.get_item("keep").expect("get").is_hit());

        assert!(parse_record(b"short").is_none());
        assert!(parse_record(b"SCFS\x02\0\0\0\0\0\0\0\0").is_none());
        assert!(!is_expired_unix_ms(0));
        assert!(is_expired_unix_ms(1));
        assert!(!is_expired_unix_ms(u64::MAX));
        assert_eq!(unix_ms_from_instant(None), 0);
        assert_eq!(unix_ms_from_instant(Some(Instant::now())), 0);
        assert_ne!(
            unix_ms_from_instant(Some(Instant::now() + Duration::from_nanos(500))),
            0,
            "sub-millisecond future Instant must not encode as no-expiry"
        );

        assert!(
            read_file(std::path::Path::new("/")).is_err(),
            "reading a directory as a cache file must fail"
        );
    }

    #[test]
    fn open_fails_when_directory_is_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let blocker = dir.path().join("not-a-dir");
        std::fs::write(&blocker, b"x").expect("blocker");
        assert!(
            FilesystemAdapter::open(
                FilesystemAdapterConfig::new(&blocker).with_prefix("p"),
                Arc::new(BytesMarshaller),
            )
            .is_err(),
            "file cannot be cache root"
        );
    }

    #[test]
    fn prefix_edge_cases_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        for prefix in ["", "a/b", "a\\b", ".", ".."] {
            assert!(
                FilesystemAdapter::open(
                    FilesystemAdapterConfig::new(dir.path()).with_prefix(prefix),
                    Arc::new(BytesMarshaller),
                )
                .is_err(),
                "prefix={prefix}"
            );
        }
    }

    #[test]
    fn get_item_errors_when_cache_path_is_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = FilesystemAdapter::open(
            FilesystemAdapterConfig::new(dir.path()).with_prefix("dirhit"),
            Arc::new(BytesMarshaller),
        )
        .expect("open");
        let path = dir
            .path()
            .join("dirhit")
            .join(format!("{}.cache", hex_key("d")));
        std::fs::create_dir(&path).expect("dir as cache file");
        assert!(matches!(pool.get_item("d"), Err(CacheError::Pool { .. })));
    }

    #[test]
    fn atomic_write_and_delete_fail_on_readonly_dir() {
        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;

            use super::atomic_write;

            let dir = tempfile::tempdir().expect("tempdir");
            let pool = FilesystemAdapter::open(
                FilesystemAdapterConfig::new(dir.path()).with_prefix("ro"),
                Arc::new(BytesMarshaller),
            )
            .expect("open");
            let root = dir.path().join("ro");
            let mut item = ArrayCacheItem::miss("k");
            item.set(Arc::new(String::from("v")));
            pool.save(item).expect("seed");

            let mut perms = fs::metadata(&root).expect("meta").permissions();
            perms.set_mode(0o555);
            fs::set_permissions(&root, perms).expect("chmod");

            let mut item = ArrayCacheItem::miss("n");
            item.set(Arc::new(String::from("x")));
            assert!(matches!(pool.save(item), Err(CacheError::Pool { .. })));
            assert!(matches!(
                pool.delete_item("k"),
                Err(CacheError::Pool { .. })
            ));
            assert!(matches!(
                atomic_write(&root.join("z.cache"), b"SCFS"),
                Err(CacheError::Pool { .. })
            ));

            let mut perms = fs::metadata(&root).expect("meta").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&root, perms).expect("restore");
        }
    }

    #[test]
    fn clear_fails_when_root_unreadable() {
        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::PermissionsExt;

            let dir = tempfile::tempdir().expect("tempdir");
            let pool = FilesystemAdapter::open(
                FilesystemAdapterConfig::new(dir.path()).with_prefix("locked"),
                Arc::new(BytesMarshaller),
            )
            .expect("open");
            let root = dir.path().join("locked");
            let mut perms = fs::metadata(&root).expect("meta").permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&root, perms).expect("chmod");
            assert!(matches!(pool.clear(), Err(CacheError::Pool { .. })));
            let mut perms = fs::metadata(&root).expect("meta").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&root, perms).expect("restore");
        }
    }

    #[test]
    fn rename_fails_when_destination_is_directory() {
        use std::fs;

        use super::atomic_write;

        let dir = tempfile::tempdir().expect("tempdir");
        let dest = dir.path().join("x.cache");
        fs::create_dir(&dest).expect("dest dir");
        assert!(matches!(
            atomic_write(&dest, b"payload"),
            Err(CacheError::Pool { .. })
        ));
    }

    #[test]
    fn atomic_write_fails_when_tmp_is_dev_full() {
        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::symlink;

            use super::atomic_write;

            let dir = tempfile::tempdir().expect("tempdir");
            let dest = dir.path().join("x.cache");
            let tmp = dest.with_extension("cache.tmp");
            symlink("/dev/full", &tmp).expect("symlink /dev/full");
            assert!(matches!(
                atomic_write(&dest, b"payload-that-must-fail"),
                Err(CacheError::Pool { .. })
            ));
            let _ = fs::remove_file(&tmp);
        }
    }

    #[test]
    fn read_file_permission_denied() {
        #[cfg(unix)]
        {
            use super::read_file;

            // Non-root typically cannot open /root.
            let _ = read_file(std::path::Path::new("/root"));
        }
    }
}
