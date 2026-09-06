//! Redis-backed [`CacheItemPool`](crate::CacheItemPool).

use std::sync::Arc;
use std::time::{Duration, Instant};

use r2d2::Pool;
use redis::{Client, Commands, RedisError};

use super::RedisAdapterConfig;
use super::keys::{redis_key, validate_logical_key};
use super::marshaller::CacheMarshaller;
use crate::{ArrayCacheItem, CacheError, CacheItemPool};

/// Redis [`CacheItemPool`] using redis-rs sync connections and an r2d2 pool.
pub struct RedisAdapter {
    pool: Pool<Client>,
    prefix: String,
    marshaller: Arc<dyn CacheMarshaller>,
}

impl RedisAdapter {
    /// Connects using `config` and `marshaller`.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError::Pool`] when the URL is invalid or the pool cannot be built.
    pub fn connect(
        config: RedisAdapterConfig,
        marshaller: Arc<dyn CacheMarshaller>,
    ) -> Result<Self, CacheError> {
        let client = Client::open(config.url.as_str()).map_err(|error| map_redis(&error))?;
        let mut builder = Pool::builder();
        builder = builder
            .max_size(config.pool_max_size)
            .connection_timeout(config.connection_timeout);
        if config.warm_pool {
            builder = builder.min_idle(Some(1));
        }
        let pool = builder.build(client).map_err(|error| CacheError::Pool {
            message: error.to_string(),
        })?;
        Ok(Self {
            pool,
            prefix: config.prefix,
            marshaller,
        })
    }

    fn connection(&self) -> Result<r2d2::PooledConnection<Client>, CacheError> {
        self.pool.get().map_err(|error| CacheError::Pool {
            message: error.to_string(),
        })
    }

    fn prefixed(&self, key: &str) -> Result<String, CacheError> {
        redis_key(&self.prefix, key)
    }
}

impl CacheItemPool for RedisAdapter {
    fn get_item(&self, key: &str) -> Result<ArrayCacheItem, CacheError> {
        let redis_key = self.prefixed(key)?;
        let mut conn = self.connection()?;
        let value: Option<Vec<u8>> = conn.get(&redis_key).map_err(|error| map_redis(&error))?;
        let Some(bytes) = value else {
            return Ok(ArrayCacheItem::miss(key));
        };
        let Ok(decoded) = self.marshaller.unmarshal(&bytes) else {
            return Ok(ArrayCacheItem::miss(key));
        };
        let pttl: i64 = conn.pttl(&redis_key).map_err(|error| map_redis(&error))?;
        Ok(item_from_pttl(key, decoded, pttl))
    }

    fn save(&self, item: ArrayCacheItem) -> Result<(), CacheError> {
        let (key, value, expires_at) = item.into_stored();
        let redis_key = self.prefixed(&key)?;
        let mut conn = self.connection()?;
        let Some(value) = value else {
            let _: () = conn.del(&redis_key).map_err(|error| map_redis(&error))?;
            return Ok(());
        };
        if expires_at.is_some_and(|at| Instant::now() >= at) {
            let _: () = conn.del(&redis_key).map_err(|error| map_redis(&error))?;
            return Ok(());
        }
        let bytes = self.marshaller.marshal(value.as_ref())?;
        match remaining_px_ms(expires_at) {
            Some(px) => {
                redis::cmd("SET")
                    .arg(&redis_key)
                    .arg(bytes)
                    .arg("PX")
                    .arg(px)
                    .query::<()>(&mut *conn)
                    .map_err(|error| map_redis(&error))?;
            }
            None => {
                let _: () = conn
                    .set(&redis_key, bytes)
                    .map_err(|error| map_redis(&error))?;
            }
        }
        Ok(())
    }

    fn delete_item(&self, key: &str) -> Result<bool, CacheError> {
        let redis_key = self.prefixed(key)?;
        let mut conn = self.connection()?;
        let removed: i32 = conn.del(&redis_key).map_err(|error| map_redis(&error))?;
        Ok(removed > 0)
    }

    fn clear(&self) -> Result<(), CacheError> {
        let pattern = format!("{}*", self.prefix);
        let mut conn = self.connection()?;
        let mut cursor: u64 = 0;
        loop {
            let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100_u64)
                .query(&mut *conn)
                .map_err(|error| map_redis(&error))?;
            if !keys.is_empty() {
                let _: () = conn.del(keys).map_err(|error| map_redis(&error))?;
            }
            cursor = next;
            if cursor == 0 {
                break;
            }
        }
        Ok(())
    }

    fn has_item(&self, key: &str) -> Result<bool, CacheError> {
        let redis_key = self.prefixed(key)?;
        let mut conn = self.connection()?;
        let exists: bool = conn.exists(&redis_key).map_err(|error| map_redis(&error))?;
        Ok(exists)
    }

    fn get_items(&self, keys: &[&str]) -> Result<Vec<ArrayCacheItem>, CacheError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        for key in keys {
            validate_logical_key(key)?;
        }
        let redis_keys: Vec<String> = keys
            .iter()
            .map(|key| redis_key(&self.prefix, key))
            .collect::<Result<Vec<_>, _>>()?;
        let mut conn = self.connection()?;
        let values: Vec<Option<Vec<u8>>> = redis::cmd("MGET")
            .arg(&redis_keys)
            .query(&mut *conn)
            .map_err(|error| map_redis(&error))?;
        let mut out = Vec::with_capacity(keys.len());
        for (logical, (full_key, value)) in keys.iter().zip(redis_keys.iter().zip(values)) {
            let Some(bytes) = value else {
                out.push(ArrayCacheItem::miss(*logical));
                continue;
            };
            let Ok(decoded) = self.marshaller.unmarshal(&bytes) else {
                out.push(ArrayCacheItem::miss(*logical));
                continue;
            };
            let pttl: i64 = conn.pttl(full_key).map_err(|error| map_redis(&error))?;
            out.push(item_from_pttl(logical, decoded, pttl));
        }
        Ok(out)
    }

    fn delete_items(&self, keys: &[&str]) -> Result<usize, CacheError> {
        if keys.is_empty() {
            return Ok(0);
        }
        let redis_keys: Vec<String> = keys
            .iter()
            .map(|key| self.prefixed(key))
            .collect::<Result<Vec<_>, _>>()?;
        let mut conn = self.connection()?;
        let removed: usize = conn.del(redis_keys).map_err(|error| map_redis(&error))?;
        Ok(removed)
    }
}

fn item_from_pttl(
    key: &str,
    decoded: Arc<dyn std::any::Any + Send + Sync>,
    pttl: i64,
) -> ArrayCacheItem {
    let item = ArrayCacheItem::hit(key, decoded);
    match pttl {
        -2 => ArrayCacheItem::miss(key),
        -1 => item,
        ms if ms > 0 => {
            let millis = u64::try_from(ms).unwrap_or(u64::MAX);
            let expires_at = Instant::now() + Duration::from_millis(millis);
            item.with_expiry(Some(expires_at))
        }
        _ => item,
    }
}

fn map_redis(error: &RedisError) -> CacheError {
    CacheError::Pool {
        message: error.to_string(),
    }
}

fn remaining_px_ms(expires_at: Option<Instant>) -> Option<u64> {
    let at = expires_at?;
    let now = Instant::now();
    if at <= now {
        return None;
    }
    Some(
        u64::try_from(at.duration_since(now).as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{item_from_pttl, remaining_px_ms};
    use crate::CacheItem;

    #[test]
    fn item_from_pttl_branches() {
        let value = Arc::new(String::from("v")) as Arc<dyn std::any::Any + Send + Sync>;
        assert!(!item_from_pttl("k", Arc::clone(&value), -2).is_hit());
        assert!(item_from_pttl("k", Arc::clone(&value), -1).is_hit());
        let with_ttl = item_from_pttl("k", Arc::clone(&value), 5_000);
        assert!(with_ttl.is_hit());
        assert!(!with_ttl.is_expired());
        assert!(item_from_pttl("k", value, 0).is_hit());
    }

    #[test]
    fn remaining_px_none_when_already_elapsed() {
        assert!(remaining_px_ms(None).is_none());
        let past = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .expect("past");
        assert!(remaining_px_ms(Some(past)).is_none());
        let future = Instant::now() + Duration::from_secs(2);
        assert!(remaining_px_ms(Some(future)).unwrap() >= 1);
    }
}

#[cfg(all(test, feature = "redis"))]
mod pool_checkout_tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::RedisAdapter;
    use crate::{BytesMarshaller, CacheItemPool, RedisAdapterConfig};

    #[test]
    fn connection_checkout_times_out_when_pool_exhausted() {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1:6379/0"));
        let config = RedisAdapterConfig::new(url)
            .with_prefix("serenade-ut:exh:")
            .with_pool_max_size(1)
            .with_connection_timeout(Duration::from_millis(150));
        let pool =
            Arc::new(RedisAdapter::connect(config, Arc::new(BytesMarshaller)).expect("connect"));
        let blocker = Arc::clone(&pool);
        let handle = thread::spawn(move || {
            let mut conn = blocker.connection().expect("hold connection");
            let _: Option<(String, String)> = redis::cmd("BLPOP")
                .arg("serenade-ut:exh:blocklist")
                .arg(2.0)
                .query(&mut *conn)
                .expect("blpop");
        });
        thread::sleep(Duration::from_millis(80));
        assert!(
            pool.connection().is_err(),
            "checkout must time out while BLPOP holds the only connection"
        );
        assert!(pool.get_item("x").is_err());
        handle.join().expect("join");
    }
}
