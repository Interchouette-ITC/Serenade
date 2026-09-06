//! Encode / decode cache values for Redis.

use std::any::Any;
use std::sync::Arc;

use crate::CacheError;

const TAG_BYTES: u8 = 1;
const TAG_STRING: u8 = 2;

/// Converts typed cache values to Redis bytes and back.
pub trait CacheMarshaller: Send + Sync {
    /// Encodes `value` for storage.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when the value type is unsupported.
    fn marshal(&self, value: &(dyn Any + Send + Sync)) -> Result<Vec<u8>, CacheError>;

    /// Decodes Redis payload into a typed value.
    ///
    /// # Errors
    ///
    /// Returns [`CacheError`] when the payload is corrupt or unsupported.
    fn unmarshal(&self, bytes: &[u8]) -> Result<Arc<dyn Any + Send + Sync>, CacheError>;
}

/// Marshaller for [`Vec<u8>`] and [`String`] values (tagged payload).
#[derive(Debug, Default, Clone, Copy)]
pub struct BytesMarshaller;

impl CacheMarshaller for BytesMarshaller {
    fn marshal(&self, value: &(dyn Any + Send + Sync)) -> Result<Vec<u8>, CacheError> {
        if let Some(bytes) = value.downcast_ref::<Vec<u8>>() {
            let mut out = Vec::with_capacity(1 + bytes.len());
            out.push(TAG_BYTES);
            out.extend_from_slice(bytes);
            return Ok(out);
        }
        if let Some(text) = value.downcast_ref::<String>() {
            let mut out = Vec::with_capacity(1 + text.len());
            out.push(TAG_STRING);
            out.extend_from_slice(text.as_bytes());
            return Ok(out);
        }
        Err(CacheError::Pool {
            message: "BytesMarshaller supports only Vec<u8> and String".to_owned(),
        })
    }

    fn unmarshal(&self, bytes: &[u8]) -> Result<Arc<dyn Any + Send + Sync>, CacheError> {
        let Some((tag, payload)) = bytes.split_first() else {
            return Err(CacheError::Pool {
                message: "empty Redis cache payload".to_owned(),
            });
        };
        match *tag {
            TAG_BYTES => Ok(Arc::new(payload.to_vec())),
            TAG_STRING => {
                let text =
                    String::from_utf8(payload.to_vec()).map_err(|error| CacheError::Pool {
                        message: format!("invalid UTF-8 String payload: {error}"),
                    })?;
                Ok(Arc::new(text))
            }
            _ => Err(CacheError::Pool {
                message: format!("unknown marshaller tag {tag}"),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BytesMarshaller, CacheMarshaller};

    #[test]
    fn round_trips_bytes_and_string() {
        let marshaller = BytesMarshaller;
        let raw = marshaller
            .marshal(&vec![1_u8, 2, 3] as &(dyn std::any::Any + Send + Sync))
            .expect("marshal bytes");
        let restored = marshaller.unmarshal(&raw).expect("unmarshal");
        assert_eq!(
            restored.downcast_ref::<Vec<u8>>().map(Vec::as_slice),
            Some(&[1, 2, 3][..])
        );

        let raw = marshaller
            .marshal(&String::from("hi") as &(dyn std::any::Any + Send + Sync))
            .expect("marshal string");
        let restored = marshaller.unmarshal(&raw).expect("unmarshal");
        assert_eq!(
            restored.downcast_ref::<String>().map(String::as_str),
            Some("hi")
        );
    }

    #[test]
    fn rejects_unsupported_type() {
        let marshaller = BytesMarshaller;
        let err = marshaller
            .marshal(&42_u32 as &(dyn std::any::Any + Send + Sync))
            .expect_err("u32 unsupported");
        assert!(err.to_string().contains("Vec<u8>"));
    }

    #[test]
    fn rejects_empty_and_bad_tag() {
        let marshaller = BytesMarshaller;
        assert!(marshaller.unmarshal(&[]).is_err());
        assert!(marshaller.unmarshal(&[9, 1, 2]).is_err());
    }
}
