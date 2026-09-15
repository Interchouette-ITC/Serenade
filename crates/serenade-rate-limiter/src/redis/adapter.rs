//! Redis-backed [`RateLimiterStorage`].

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use r2d2::Pool;
use redis::{Client, Commands, RedisError, Script};

use super::RedisRateLimiterStorageConfig;
use super::wire::{decode_state, encode_state, unix_millis_now};
use crate::{Policy, RateLimit, RateLimiterError, RateLimiterStorage, validate_key};

/// Multi-node [`RateLimiterStorage`] using redis-rs sync connections and an r2d2 pool.
///
/// Window timestamps are stored as unix millis; [`Instant`] values are bridged at this boundary.
pub struct RedisRateLimiterStorage {
    pool: Pool<Client>,
    prefix: String,
    /// Test-only: force the next N compare-and-set attempts to fail.
    cas_fail_budget: AtomicU32,
}

impl RedisRateLimiterStorage {
    /// Connects using `config`.
    ///
    /// # Errors
    ///
    /// Returns [`RateLimiterError::Storage`] when the URL is invalid or the pool cannot be built.
    pub fn connect(config: RedisRateLimiterStorageConfig) -> Result<Self, RateLimiterError> {
        let client = Client::open(config.url.as_str()).map_err(|error| map_redis(&error))?;
        let mut builder = Pool::builder();
        builder = builder
            .max_size(config.pool_max_size)
            .connection_timeout(config.connection_timeout);
        if config.warm_pool {
            builder = builder.min_idle(Some(1));
        }
        let pool = builder
            .build(client)
            .map_err(|error| RateLimiterError::Storage {
                message: error.to_string(),
            })?;
        Ok(Self {
            pool,
            prefix: config.prefix,
            cas_fail_budget: AtomicU32::new(0),
        })
    }

    /// Redis key prefix used by this storage.
    #[must_use]
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    fn connection(&self) -> Result<r2d2::PooledConnection<Client>, RateLimiterError> {
        self.pool.get().map_err(|error| RateLimiterError::Storage {
            message: error.to_string(),
        })
    }

    fn redis_key(&self, id: &str) -> String {
        format!("{}{id}", self.prefix)
    }
}

impl RateLimiterStorage for RedisRateLimiterStorage {
    fn consume(
        &self,
        id: &str,
        policy: &Policy,
        tokens: u32,
        now: Instant,
        ttl: Option<Duration>,
    ) -> Result<RateLimit, RateLimiterError> {
        validate_key(id)?;
        let key = self.redis_key(id);
        let now_wall_ms = unix_millis_now();

        for _ in 0..32 {
            let mut conn = self.connection()?;
            let raw: Option<String> = conn.get(&key).map_err(|error| map_redis(&error))?;
            let observed = raw.clone();
            let mut state = match raw.as_deref() {
                None => None,
                Some(value) => Some(decode_state(value, now, now_wall_ms)?),
            };
            let result = policy.consume(&mut state, tokens, now)?;
            let window = state.expect("policy leaves window state after consume");
            let encoded = encode_state(&window, now, now_wall_ms);
            let cas_ok = apply_window(&mut conn, &key, observed.as_deref(), &encoded, ttl)?;
            let forced_fail = self
                .cas_fail_budget
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1))
                .is_ok();
            if cas_ok && !forced_fail {
                return Ok(result);
            }
        }
        Err(RateLimiterError::Storage {
            message: "redis compare-and-set retries exhausted".to_owned(),
        })
    }

    fn reset(&self, id: &str) -> Result<(), RateLimiterError> {
        validate_key(id)?;
        let key = self.redis_key(id);
        let mut conn = self.connection()?;
        let _: () = conn.del(key).map_err(|error| map_redis(&error))?;
        Ok(())
    }
}

fn apply_window(
    conn: &mut r2d2::PooledConnection<Client>,
    key: &str,
    expected: Option<&str>,
    next: &str,
    ttl: Option<Duration>,
) -> Result<bool, RateLimiterError> {
    // CAS: only write when the key still matches `expected` (or is absent when expected is None).
    let script = Script::new(
        r"
        local cur = redis.call('GET', KEYS[1])
        local expected = ARGV[1]
        local next = ARGV[2]
        local ttl = tonumber(ARGV[3])
        if expected == '' then
          if cur ~= false then return 0 end
        else
          if cur ~= expected then return 0 end
        end
        redis.call('SET', KEYS[1], next)
        if ttl > 0 then
          redis.call('PEXPIRE', KEYS[1], ttl)
        else
          redis.call('PERSIST', KEYS[1])
        end
        return 1
        ",
    );
    let expected_arg = expected.unwrap_or("");
    let ttl_ms = ttl.map_or(0_i64, duration_to_px_ms);
    let applied: i32 = script
        .key(key)
        .arg(expected_arg)
        .arg(next)
        .arg(ttl_ms)
        .invoke(conn)
        .map_err(|error| map_redis(&error))?;
    Ok(applied == 1)
}

fn duration_to_px_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis())
        .unwrap_or(i64::MAX)
        .max(1)
}

fn map_redis(error: &RedisError) -> RateLimiterError {
    RateLimiterError::Storage {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    use r2d2::Pool;
    use redis::{Client, Commands};

    use super::{RedisRateLimiterStorage, apply_window};
    use crate::{Policy, RateLimiterError, RateLimiterStorage};

    #[test]
    fn pool_checkout_timeout_maps_to_storage_error() {
        let storage = RedisRateLimiterStorage {
            pool: Pool::builder()
                .max_size(1)
                .connection_timeout(Duration::from_millis(200))
                .build(Client::open("redis://127.0.0.1:6379/0").expect("client"))
                .expect("pool"),
            prefix: format!(
                "serenade:rate-limiter:test:pool:{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ),
            cas_fail_budget: std::sync::atomic::AtomicU32::new(0),
        };
        let _held = storage.pool.get().expect("hold sole connection");
        let policy = Policy::fixed_window(1, Duration::from_secs(60)).expect("policy");
        assert!(matches!(
            storage.consume("k", &policy, 1, Instant::now(), None),
            Err(RateLimiterError::Storage { .. })
        ));
    }

    #[test]
    fn compare_and_set_conflict_and_retries_exhausted() {
        let prefix = format!(
            "serenade:rate-limiter:test:cas:{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let storage = RedisRateLimiterStorage::connect(
            super::super::RedisRateLimiterStorageConfig::new("redis://127.0.0.1:6379/0")
                .with_prefix(&prefix),
        )
        .expect("connect");
        let key = storage.redis_key("subject");
        let mut conn = storage.connection().expect("conn");
        let _: () = conn.set(&key, "fw:1:1").expect("seed");
        assert!(!apply_window(&mut conn, &key, Some("other"), "fw:2:2", None).expect("cas"));
        assert!(
            apply_window(
                &mut conn,
                &key,
                Some("fw:1:1"),
                "fw:2:2",
                Some(Duration::from_secs(5)),
            )
            .expect("cas ok")
        );

        storage.cas_fail_budget.store(64, Ordering::Relaxed);
        let policy = Policy::fixed_window(5, Duration::from_secs(60)).expect("policy");
        let err = storage
            .consume("subject", &policy, 1, Instant::now(), None)
            .expect_err("exhausted");
        assert!(
            matches!(err, RateLimiterError::Storage { message } if message.contains("retries"))
        );
    }
}
