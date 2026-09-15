//! Redis-backed [`WireEnvelope`](crate::WireEnvelope) queue (RPUSH / LPOP).

use std::future::Future;
use std::pin::Pin;

use r2d2::Pool;
use redis::{Client, Commands, RedisError};

use super::RedisTransportConfig;
use crate::{MessengerError, WireEnvelope};

/// Durable Redis list transport for wire frames.
///
/// Does **not** implement in-process [`crate::Transport`] (`Envelope` is `Any` and cannot
/// cross the wire). Apps serialize payloads, then [`Self::send_wire`] / [`Self::receive_wire`].
pub struct RedisTransport {
    pool: Pool<Client>,
    list_key: String,
}

impl RedisTransport {
    /// Connects using `config`.
    ///
    /// # Errors
    ///
    /// Returns [`MessengerError::Transport`] when the URL is invalid or the pool cannot be built.
    pub fn connect(config: RedisTransportConfig) -> Result<Self, MessengerError> {
        if config.list_key.is_empty() {
            return Err(MessengerError::Transport {
                message: "redis messenger list key must not be empty".to_owned(),
            });
        }
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
            .map_err(|error| MessengerError::Transport {
                message: error.to_string(),
            })?;
        Ok(Self {
            pool,
            list_key: config.list_key,
        })
    }

    /// Redis list key used by this transport.
    #[must_use]
    pub fn list_key(&self) -> &str {
        &self.list_key
    }

    fn connection(&self) -> Result<r2d2::PooledConnection<Client>, MessengerError> {
        self.pool.get().map_err(|error| MessengerError::Transport {
            message: error.to_string(),
        })
    }

    /// Enqueues `envelope` at the tail of the Redis list (RPUSH).
    pub fn send_wire(
        &self,
        envelope: WireEnvelope,
    ) -> Pin<Box<dyn Future<Output = Result<(), MessengerError>> + Send + '_>> {
        Box::pin(async move {
            let encoded = envelope.encode();
            let mut conn = self.connection()?;
            let _: () = conn
                .rpush(&self.list_key, encoded)
                .map_err(|error| map_redis(&error))?;
            Ok(())
        })
    }

    /// Pops the next frame from the head of the list (LPOP), if any.
    pub fn receive_wire(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Option<WireEnvelope>, MessengerError>> + Send + '_>>
    {
        Box::pin(async move {
            let mut conn = self.connection()?;
            let value: Option<Vec<u8>> = conn
                .lpop(&self.list_key, None)
                .map_err(|error| map_redis(&error))?;
            match value {
                None => Ok(None),
                Some(bytes) => Ok(Some(WireEnvelope::decode(&bytes)?)),
            }
        })
    }

    /// Number of frames waiting on the list.
    ///
    /// # Errors
    ///
    /// Returns [`MessengerError::Transport`] on Redis failure.
    pub fn len(&self) -> Result<usize, MessengerError> {
        let mut conn = self.connection()?;
        let len: usize = conn
            .llen(&self.list_key)
            .map_err(|error| map_redis(&error))?;
        Ok(len)
    }

    /// Whether the list is empty.
    ///
    /// # Errors
    ///
    /// Returns [`MessengerError::Transport`] on Redis failure.
    pub fn is_empty(&self) -> Result<bool, MessengerError> {
        Ok(self.len()? == 0)
    }
}

fn map_redis(error: &RedisError) -> MessengerError {
    MessengerError::Transport {
        message: error.to_string(),
    }
}
