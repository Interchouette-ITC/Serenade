//! Durable wire frame: message name + opaque payload bytes.

use crate::MessengerError;

/// Cross-process messenger frame (apps own serialization of `payload`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WireEnvelope {
    message_name: String,
    payload: Vec<u8>,
}

impl WireEnvelope {
    /// Builds a wire frame.
    ///
    /// # Errors
    ///
    /// Returns [`MessengerError::Transport`] when `message_name` is empty.
    pub fn new(
        message_name: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, MessengerError> {
        let message_name = message_name.into();
        if message_name.is_empty() {
            return Err(MessengerError::Transport {
                message: "wire envelope message name must not be empty".to_owned(),
            });
        }
        Ok(Self {
            message_name,
            payload: payload.into(),
        })
    }

    /// Message name (stable routing key for workers).
    #[must_use]
    pub fn message_name(&self) -> &str {
        &self.message_name
    }

    /// Opaque payload bytes.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Encodes this frame for a Redis list value.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let name = self.message_name.as_bytes();
        let name_len = u32::try_from(name.len()).unwrap_or(u32::MAX);
        let payload_len = u32::try_from(self.payload.len()).unwrap_or(u32::MAX);
        let mut out = Vec::with_capacity(4 + 4 + 4 + name.len() + self.payload.len());
        out.extend_from_slice(b"SERW");
        out.extend_from_slice(&name_len.to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(&payload_len.to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    /// Decodes a Redis list value produced by [`Self::encode`].
    ///
    /// # Errors
    ///
    /// Returns [`MessengerError::Transport`] when the frame is truncated or invalid.
    pub fn decode(bytes: &[u8]) -> Result<Self, MessengerError> {
        if bytes.len() < 8 || &bytes[..4] != b"SERW" {
            return Err(MessengerError::Transport {
                message: "invalid wire envelope magic".to_owned(),
            });
        }
        let name_len_bytes: [u8; 4] =
            bytes[4..8]
                .try_into()
                .map_err(|_| MessengerError::Transport {
                    message: "wire envelope truncated at name length".to_owned(),
                })?;
        let name_len = usize::try_from(u32::from_be_bytes(name_len_bytes)).map_err(|_| {
            MessengerError::Transport {
                message: "wire envelope name length does not fit usize".to_owned(),
            }
        })?;
        let name_end = 8usize
            .checked_add(name_len)
            .ok_or_else(|| MessengerError::Transport {
                message: "wire envelope name length overflow".to_owned(),
            })?;
        if bytes.len() < name_end + 4 {
            return Err(MessengerError::Transport {
                message: "wire envelope truncated at name".to_owned(),
            });
        }
        let message_name = String::from_utf8(bytes[8..name_end].to_vec()).map_err(|error| {
            MessengerError::Transport {
                message: format!("wire envelope name is not utf-8: {error}"),
            }
        })?;
        let payload_len_offset = name_end;
        let payload_len_bytes: [u8; 4] = bytes[payload_len_offset..payload_len_offset + 4]
            .try_into()
            .map_err(|_| MessengerError::Transport {
                message: "wire envelope truncated at payload length".to_owned(),
            })?;
        let payload_len = usize::try_from(u32::from_be_bytes(payload_len_bytes)).map_err(|_| {
            MessengerError::Transport {
                message: "wire envelope payload length does not fit usize".to_owned(),
            }
        })?;
        let payload_start = payload_len_offset + 4;
        let payload_end =
            payload_start
                .checked_add(payload_len)
                .ok_or_else(|| MessengerError::Transport {
                    message: "wire envelope payload length overflow".to_owned(),
                })?;
        if bytes.len() != payload_end {
            return Err(MessengerError::Transport {
                message: "wire envelope length mismatch".to_owned(),
            });
        }
        Self::new(message_name, bytes[payload_start..payload_end].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::WireEnvelope;

    #[test]
    fn round_trip_and_rejects_empty_name() {
        assert!(WireEnvelope::new("", b"x").is_err());
        let wire = WireEnvelope::new("demo.command", b"{\"id\":1}").expect("wire");
        let decoded = WireEnvelope::decode(&wire.encode()).expect("decode");
        assert_eq!(decoded.message_name(), "demo.command");
        assert_eq!(decoded.payload(), b"{\"id\":1}");
    }

    #[test]
    fn decode_rejects_bad_magic() {
        assert!(WireEnvelope::decode(b"XXXX").is_err());
    }
}
