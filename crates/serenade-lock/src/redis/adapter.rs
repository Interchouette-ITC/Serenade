//! Redis-backed [`LockStore`].

use std::time::Duration;

use r2d2::Pool;
use redis::{Client, Commands, RedisError};

use super::RedisLockStoreConfig;
use crate::{LockError, LockStore, validate_resource};

/// Redis [`LockStore`] using redis-rs sync connections and an r2d2 pool.
pub struct RedisLockStore {
    pool: Pool<Client>,
    prefix: String,
}

impl RedisLockStore {
    /// Connects using `config`.
    ///
    /// # Errors
    ///
    /// Returns [`LockError::Store`] when the URL is invalid or the pool cannot be built.
    pub fn connect(config: RedisLockStoreConfig) -> Result<Self, LockError> {
        let client = Client::open(config.url.as_str()).map_err(|error| map_redis(&error))?;
        let mut builder = Pool::builder();
        builder = builder
            .max_size(config.pool_max_size)
            .connection_timeout(config.connection_timeout);
        if config.warm_pool {
            builder = builder.min_idle(Some(1));
        }
        let pool = builder.build(client).map_err(|error| LockError::Store {
            message: error.to_string(),
        })?;
        Ok(Self {
            pool,
            prefix: config.prefix,
        })
    }

    fn connection(&self) -> Result<r2d2::PooledConnection<Client>, LockError> {
        self.pool.get().map_err(|error| LockError::Store {
            message: error.to_string(),
        })
    }

    fn redis_key(&self, resource: &str) -> String {
        format!("{}{resource}", self.prefix)
    }

    #[cfg(test)]
    pub(crate) const fn pool_for_test(&self) -> &Pool<Client> {
        &self.pool
    }
}

impl LockStore for RedisLockStore {
    fn save(&self, resource: &str, token: &str, ttl: Option<Duration>) -> Result<(), LockError> {
        validate_resource(resource)?;
        let key = self.redis_key(resource);
        let mut conn = self.connection()?;
        let current: Option<String> = conn.get(&key).map_err(|error| map_redis(&error))?;
        match current.as_deref() {
            Some(held) if held == token => {
                set_token(&mut conn, &key, token, ttl, false)?;
                Ok(())
            }
            Some(_) => Err(LockError::Conflict {
                key: resource.to_owned(),
            }),
            None => {
                if set_token(&mut conn, &key, token, ttl, true)? {
                    Ok(())
                } else {
                    Err(LockError::Conflict {
                        key: resource.to_owned(),
                    })
                }
            }
        }
    }

    fn delete(&self, resource: &str, token: &str) -> Result<(), LockError> {
        validate_resource(resource)?;
        let key = self.redis_key(resource);
        let mut conn = self.connection()?;
        let current: Option<String> = conn.get(&key).map_err(|error| map_redis(&error))?;
        if current.as_deref() == Some(token) {
            let _: () = conn.del(&key).map_err(|error| map_redis(&error))?;
        }
        Ok(())
    }

    fn exists(&self, resource: &str, token: &str) -> Result<bool, LockError> {
        validate_resource(resource)?;
        let key = self.redis_key(resource);
        let mut conn = self.connection()?;
        let current: Option<String> = conn.get(&key).map_err(|error| map_redis(&error))?;
        Ok(current.as_deref() == Some(token))
    }

    fn put_off_expiration(
        &self,
        resource: &str,
        token: &str,
        ttl: Option<Duration>,
    ) -> Result<(), LockError> {
        validate_resource(resource)?;
        let key = self.redis_key(resource);
        let mut conn = self.connection()?;
        let current: Option<String> = conn.get(&key).map_err(|error| map_redis(&error))?;
        if current.as_deref() != Some(token) {
            return Err(LockError::NotHeld {
                key: resource.to_owned(),
            });
        }
        match ttl {
            Some(duration) => {
                let millis = duration_to_px_ms(duration);
                let _: bool = conn
                    .pexpire(&key, millis)
                    .map_err(|error| map_redis(&error))?;
            }
            None => {
                let _: bool = conn.persist(&key).map_err(|error| map_redis(&error))?;
            }
        }
        Ok(())
    }
}

fn set_token(
    conn: &mut r2d2::PooledConnection<Client>,
    key: &str,
    token: &str,
    ttl: Option<Duration>,
    only_if_absent: bool,
) -> Result<bool, LockError> {
    let mut cmd = redis::cmd("SET");
    cmd.arg(key).arg(token);
    if only_if_absent {
        cmd.arg("NX");
    }
    if let Some(duration) = ttl {
        cmd.arg("PX").arg(duration_to_px_ms(duration));
    }
    if only_if_absent {
        let result: Option<String> = cmd.query(conn).map_err(|error| map_redis(&error))?;
        Ok(result.is_some())
    } else {
        let _: () = cmd.query(conn).map_err(|error| map_redis(&error))?;
        Ok(true)
    }
}

fn duration_to_px_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis())
        .unwrap_or(i64::MAX)
        .max(1)
}

fn map_redis(error: &RedisError) -> LockError {
    LockError::Store {
        message: error.to_string(),
    }
}
