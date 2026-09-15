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
const VERSION_V1: u8 = 1;
const VERSION_V2: u8 = 2;
const HEADER_V1_LEN: usize = 4 + 1 + 8;

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
/// every `*.cache` file under the adapter directory (and the `.tags` index).
/// Values must be marshallable (default: [`crate::BytesMarshaller`] for `String` /
/// `Vec<u8>`). Invalidation tags are persisted in the record and indexed under `.tags/`.
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

    fn tags_dir(&self) -> PathBuf {
        self.root.join(".tags")
    }

    fn tag_index_path(&self, tag: &str) -> PathBuf {
        self.tags_dir().join(format!("{}.idx", hex_key(tag)))
    }

    fn read_tags_from_path(path: &Path) -> Result<Vec<String>, CacheError> {
        let Some(bytes) = read_file(path)? else {
            return Ok(Vec::new());
        };
        Ok(parse_record(&bytes)
            .map(|(_, tags, _)| tags)
            .unwrap_or_default())
    }

    fn unlink_key_from_tags(&self, key: &str, tags: &[String]) -> Result<(), CacheError> {
        let encoded_key = hex_key(key);
        for tag in tags.iter().filter(|tag| !tag.is_empty()) {
            let path = self.tag_index_path(tag);
            let Some(raw) = read_file(&path)? else {
                continue;
            };
            let text = String::from_utf8_lossy(&raw);
            let mut kept = Vec::new();
            for line in text.lines() {
                if line != encoded_key && !line.is_empty() {
                    kept.push(line.to_owned());
                }
            }
            if kept.is_empty() {
                let _ = fs::remove_file(&path);
            } else {
                let body = kept.join("\n");
                atomic_write(&path, body.as_bytes())?;
            }
        }
        Ok(())
    }

    fn link_key_to_tags(&self, key: &str, tags: &[String]) -> Result<(), CacheError> {
        if tags.is_empty() {
            return Ok(());
        }
        fs::create_dir_all(self.tags_dir()).map_err(|error| CacheError::Pool {
            message: format!("create tags dir: {error}"),
        })?;
        let encoded_key = hex_key(key);
        for tag in tags.iter().filter(|tag| !tag.is_empty()) {
            let path = self.tag_index_path(tag);
            let mut lines = Vec::new();
            if let Some(raw) = read_file(&path)? {
                for line in String::from_utf8_lossy(&raw).lines() {
                    if !line.is_empty() && line != encoded_key {
                        lines.push(line.to_owned());
                    }
                }
            }
            lines.push(encoded_key.clone());
            atomic_write(&path, lines.join("\n").as_bytes())?;
        }
        Ok(())
    }

    fn remove_stored_key(&self, key: &str) -> Result<bool, CacheError> {
        let path = self.path_for(key)?;
        let tags = Self::read_tags_from_path(&path)?;
        match fs::remove_file(&path) {
            Ok(()) => {
                self.unlink_key_from_tags(key, &tags)?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(CacheError::Pool {
                message: format!("delete {}: {error}", path.display()),
            }),
        }
    }
}

impl CacheItemPool for FilesystemAdapter {
    fn get_item(&self, key: &str) -> Result<ArrayCacheItem, CacheError> {
        let path = self.path_for(key)?;
        let Some(bytes) = read_file(&path)? else {
            return Ok(ArrayCacheItem::miss(key));
        };
        let Some((expires_unix_ms, tags, payload)) = parse_record(&bytes) else {
            let _ = fs::remove_file(&path);
            return Ok(ArrayCacheItem::miss(key));
        };
        if is_expired_unix_ms(expires_unix_ms) {
            let _ = self.remove_stored_key(key)?;
            return Ok(ArrayCacheItem::miss(key));
        }
        let Ok(decoded) = self.marshaller.unmarshal(payload) else {
            let _ = self.remove_stored_key(key)?;
            return Ok(ArrayCacheItem::miss(key));
        };
        Ok(ArrayCacheItem::hit(key, decoded)
            .with_expiry(instant_from_unix_ms(expires_unix_ms))
            .with_tags(tags))
    }

    fn save(&self, item: ArrayCacheItem) -> Result<(), CacheError> {
        let (key, value, expires_at, tags) = item.into_stored();
        let path = self.path_for(&key)?;
        let previous_tags = Self::read_tags_from_path(&path)?;
        let Some(value) = value else {
            let _ = self.remove_stored_key(&key)?;
            return Ok(());
        };
        if expires_at.is_some_and(|at| Instant::now() >= at) {
            let _ = self.remove_stored_key(&key)?;
            return Ok(());
        }
        self.unlink_key_from_tags(&key, &previous_tags)?;
        let payload = self.marshaller.marshal(value.as_ref())?;
        let expires_unix_ms = unix_ms_from_instant(expires_at);
        let record = encode_record_with_tags(expires_unix_ms, &tags, &payload);
        atomic_write(&path, &record)?;
        self.link_key_to_tags(&key, &tags)?;
        Ok(())
    }

    fn delete_item(&self, key: &str) -> Result<bool, CacheError> {
        self.remove_stored_key(key)
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
        let tags_dir = self.tags_dir();
        if tags_dir.is_dir() {
            let _ = fs::remove_dir_all(&tags_dir);
        }
        Ok(())
    }

    fn invalidate_tags(&self, tags: &[&str]) -> Result<usize, CacheError> {
        let mut keys = std::collections::HashSet::new();
        for tag in tags.iter().filter(|tag| !tag.is_empty()) {
            let path = self.tag_index_path(tag);
            let Some(raw) = read_file(&path)? else {
                continue;
            };
            for line in String::from_utf8_lossy(&raw).lines() {
                if line.is_empty() {
                    continue;
                }
                if let Some(key) = unhex_key(line) {
                    keys.insert(key);
                }
            }
        }
        let mut removed = 0;
        for key in keys {
            if self.remove_stored_key(&key)? {
                removed += 1;
            }
        }
        Ok(removed)
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

fn unhex_key(hex: &str) -> Option<String> {
    if hex.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let chars: Vec<char> = hex.chars().collect();
    for chunk in chars.chunks(2) {
        let high = chunk[0].to_digit(16)?;
        let low = chunk[1].to_digit(16)?;
        bytes.push(u8::try_from((high << 4) | low).ok()?);
    }
    String::from_utf8(bytes).ok()
}

fn encode_record_with_tags(expires_unix_ms: u64, tags: &[String], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_V1_LEN + 2 + payload.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION_V2);
    out.extend_from_slice(&expires_unix_ms.to_le_bytes());
    let tag_count = u16::try_from(tags.len()).unwrap_or(u16::MAX);
    out.extend_from_slice(&tag_count.to_le_bytes());
    for tag in tags.iter().take(usize::from(tag_count)) {
        let bytes = tag.as_bytes();
        let len = u16::try_from(bytes.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&bytes[..usize::from(len)]);
    }
    out.extend_from_slice(payload);
    out
}

#[cfg(test)]
fn encode_record(expires_unix_ms: u64, payload: &[u8]) -> Vec<u8> {
    encode_record_with_tags(expires_unix_ms, &[], payload)
}

fn parse_record(bytes: &[u8]) -> Option<(u64, Vec<String>, &[u8])> {
    if bytes.len() < HEADER_V1_LEN || &bytes[..4] != MAGIC {
        return None;
    }
    let version = bytes[4];
    let mut expiry_bytes = [0_u8; 8];
    expiry_bytes.copy_from_slice(&bytes[5..13]);
    let expires_unix_ms = u64::from_le_bytes(expiry_bytes);
    match version {
        VERSION_V1 => Some((expires_unix_ms, Vec::new(), &bytes[HEADER_V1_LEN..])),
        VERSION_V2 => {
            if bytes.len() < HEADER_V1_LEN + 2 {
                return None;
            }
            let mut count_bytes = [0_u8; 2];
            count_bytes.copy_from_slice(&bytes[HEADER_V1_LEN..HEADER_V1_LEN + 2]);
            let tag_count = usize::from(u16::from_le_bytes(count_bytes));
            let mut offset = HEADER_V1_LEN + 2;
            let mut tags = Vec::with_capacity(tag_count);
            for _ in 0..tag_count {
                if offset + 2 > bytes.len() {
                    return None;
                }
                let mut len_bytes = [0_u8; 2];
                len_bytes.copy_from_slice(&bytes[offset..offset + 2]);
                let len = usize::from(u16::from_le_bytes(len_bytes));
                offset += 2;
                if offset + len > bytes.len() {
                    return None;
                }
                let tag = String::from_utf8(bytes[offset..offset + len].to_vec()).ok()?;
                offset += len;
                tags.push(tag);
            }
            Some((expires_unix_ms, tags, &bytes[offset..]))
        }
        _ => None,
    }
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
        let (_expires, _tags, payload) = parse_record(&bytes).expect("parse");
        // Force a past unix expiry on disk (no sleep / no CI scheduling flake).
        std::fs::write(&path, encode_record(1, payload)).expect("rewrite expired");
        assert!(!pool.get_item("tmp").expect("get").is_hit());
        assert!(!path.exists());
    }

    #[test]
    fn tag_invalidation_and_overwrite_without_tags() {
        let (_dir, pool) = open_pool();
        let mut a = ArrayCacheItem::miss("product:1");
        a.set(Arc::new(String::from("one")));
        a.tag(&["product", "catalog"]);
        pool.save(a).expect("save a");
        let mut b = ArrayCacheItem::miss("product:2");
        b.set(Arc::new(String::from("two")));
        b.tag(&["product"]);
        pool.save(b).expect("save b");
        assert_eq!(
            pool.get_item("product:1").expect("get").tags(),
            &["product".to_owned(), "catalog".to_owned()]
        );
        assert_eq!(pool.invalidate_tags(&["product"]).expect("inv"), 2);
        assert!(!pool.get_item("product:1").expect("get").is_hit());

        let mut c = ArrayCacheItem::miss("k");
        c.set(Arc::new(String::from("v")));
        c.tag(&["keep"]);
        pool.save(c).expect("save");
        let mut overwrite = ArrayCacheItem::miss("k");
        overwrite.set(Arc::new(String::from("v2")));
        pool.save(overwrite).expect("overwrite");
        assert_eq!(pool.get_item("k").expect("get").tags(), &[] as &[String]);
        assert_eq!(pool.invalidate_tags(&["keep"]).expect("gone"), 0);
    }

    #[test]
    fn blank_idx_lines_and_empty_tag_ignored() {
        let (dir, pool) = open_pool();
        let mut item = ArrayCacheItem::miss("k");
        item.set(Arc::new(String::from("v")));
        item.tag(&["", "live"]);
        pool.save(item).expect("save");
        assert_eq!(
            pool.get_item("k").expect("get").tags(),
            &[String::from("live")]
        );
        let idx = dir
            .path()
            .join("pool")
            .join(".tags")
            .join(format!("{}.idx", hex_key("live")));
        let mut body = std::fs::read_to_string(&idx).expect("read idx");
        body.push_str("\n\nzz\n");
        std::fs::write(&idx, body).expect("pad idx");
        assert_eq!(pool.invalidate_tags(&["", "live"]).expect("inv"), 1);
        assert!(!pool.get_item("k").expect("get").is_hit());
    }

    #[test]
    fn unhex_round_trip() {
        use super::unhex_key;
        let key = "product:42";
        assert_eq!(unhex_key(&hex_key(key)).as_deref(), Some(key));
        assert!(unhex_key("abc").is_none());
        assert!(unhex_key("zz").is_none());
    }

    #[test]
    fn parse_v1_and_truncated_v2_and_clear_tags_dir() {
        use super::{
            HEADER_V1_LEN, VERSION_V1, encode_record, encode_record_with_tags, parse_record,
        };

        let mut v1 = b"SCFS".to_vec();
        v1.push(VERSION_V1);
        v1.extend_from_slice(&0_u64.to_le_bytes());
        v1.extend_from_slice(b"hi");
        let (expires, tags, payload) = parse_record(&v1).expect("v1");
        assert_eq!(expires, 0);
        assert_eq!(tags, Vec::<String>::new());
        assert_eq!(payload, b"hi");

        assert!(parse_record(b"SCFS\x02\0\0\0\0\0\0\0\0").is_none());
        let mut truncated = encode_record(0, b"x");
        truncated.truncate(HEADER_V1_LEN + 2);
        // claim one tag then omit body
        truncated[HEADER_V1_LEN] = 1;
        truncated[HEADER_V1_LEN + 1] = 0;
        assert!(parse_record(&truncated).is_none());

        // Tag length larger than remaining bytes.
        let mut short_tag = encode_record_with_tags(0, &[String::from("ab")], b"x");
        short_tag.truncate(HEADER_V1_LEN + 2 + 2 + 1);
        assert!(parse_record(&short_tag).is_none());

        // Invalid UTF-8 inside a tag length.
        let mut bad_utf8 = encode_record(0, b"x");
        // Rewrite as one tag of length 1 with invalid byte 0xFF.
        bad_utf8.truncate(HEADER_V1_LEN);
        bad_utf8.extend_from_slice(&1_u16.to_le_bytes());
        bad_utf8.extend_from_slice(&1_u16.to_le_bytes());
        bad_utf8.push(0xFF);
        bad_utf8.extend_from_slice(b"x");
        assert!(parse_record(&bad_utf8).is_none());

        // Unknown format version.
        let mut bad_ver = b"SCFS".to_vec();
        bad_ver.push(9);
        bad_ver.extend_from_slice(&0_u64.to_le_bytes());
        assert!(parse_record(&bad_ver).is_none());

        let (dir, pool) = open_pool();
        let mut item = ArrayCacheItem::miss("t");
        item.set(Arc::new(String::from("v")));
        item.tag(&["z"]);
        pool.save(item).expect("save");
        pool.clear().expect("clear removes .tags");
        assert!(!pool.get_item("t").expect("get").is_hit());

        // Stale tag name with missing index file is ignored on delete.
        let mut item = ArrayCacheItem::miss("orphan");
        item.set(Arc::new(String::from("v")));
        item.tag(&["gone"]);
        pool.save(item).expect("save");
        let idx = dir
            .path()
            .join("pool")
            .join(".tags")
            .join(format!("{}.idx", hex_key("gone")));
        std::fs::remove_file(&idx).expect("drop idx");
        assert!(pool.delete_item("orphan").expect("delete"));
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
        assert!(parse_record(b"SCFS\x63\0\0\0\0\0\0\0\0").is_none());
        assert!(!is_expired_unix_ms(0));
        assert!(is_expired_unix_ms(1));
        assert!(!is_expired_unix_ms(u64::MAX));
        assert_eq!(unix_ms_from_instant(None), 0);
        assert_eq!(unix_ms_from_instant(Some(Instant::now())), 0);
        // 1ms stays ahead of Instant::now() under llvm-cov (sub-ms deltas flake there).
        assert_ne!(
            unix_ms_from_instant(Some(Instant::now() + Duration::from_millis(1))),
            0,
            "near-future Instant must not encode as no-expiry"
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
