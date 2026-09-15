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
        let mut name_len_bytes = [0_u8; 4];
        name_len_bytes.copy_from_slice(&bytes[4..8]);
        let name_len = usize::try_from(u32::from_be_bytes(name_len_bytes)).unwrap_or(usize::MAX);
        if name_len > bytes.len().saturating_sub(8) {
            return Err(MessengerError::Transport {
                message: "wire envelope truncated at name".to_owned(),
            });
        }
        let name_end = 8 + name_len;
        if bytes.len() < name_end + 4 {
            return Err(MessengerError::Transport {
                message: "wire envelope truncated at name".to_owned(),
            });
        }
        let payload_hdr_end = name_end + 4;
        let message_name = String::from_utf8(bytes[8..name_end].to_vec()).map_err(|error| {
            MessengerError::Transport {
                message: format!("wire envelope name is not utf-8: {error}"),
            }
        })?;
        let mut payload_len_bytes = [0_u8; 4];
        payload_len_bytes.copy_from_slice(&bytes[name_end..payload_hdr_end]);
        let payload_len =
            usize::try_from(u32::from_be_bytes(payload_len_bytes)).unwrap_or(usize::MAX);
        let payload_body_len = bytes.len() - payload_hdr_end;
        if payload_len != payload_body_len {
            return Err(MessengerError::Transport {
                message: "wire envelope length mismatch".to_owned(),
            });
        }
        Self::new(message_name, bytes[payload_hdr_end..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::WireEnvelope;
    use crate::MessengerError;

    #[test]
    fn round_trip_and_rejects_empty_name() {
        assert!(WireEnvelope::new("", b"x").is_err());
        let wire = WireEnvelope::new("demo.command", b"{\"id\":1}").expect("wire");
        let decoded = WireEnvelope::decode(&wire.encode()).expect("decode");
        assert_eq!(decoded.message_name(), "demo.command");
        assert_eq!(decoded.payload(), b"{\"id\":1}");
    }

    #[test]
    fn decode_rejects_bad_magic_and_short_header() {
        assert!(WireEnvelope::decode(b"XXXX").is_err());
        assert!(WireEnvelope::decode(b"SER").is_err());
        assert!(WireEnvelope::decode(b"SERW").is_err());
    }

    #[test]
    fn decode_rejects_truncated_name_and_length_mismatch() {
        let mut bytes = b"SERW".to_vec();
        bytes.extend_from_slice(&5_u32.to_be_bytes());
        bytes.extend_from_slice(b"ab");
        assert!(matches!(
            WireEnvelope::decode(&bytes),
            Err(MessengerError::Transport { .. })
        ));

        let mut missing_payload_hdr = b"SERW".to_vec();
        missing_payload_hdr.extend_from_slice(&1_u32.to_be_bytes());
        missing_payload_hdr.push(b'n');
        missing_payload_hdr.extend_from_slice(&[0_u8, 0_u8]);
        assert!(matches!(
            WireEnvelope::decode(&missing_payload_hdr),
            Err(MessengerError::Transport { message }) if message.contains("truncated")
        ));

        let mut good = WireEnvelope::new("n", b"p").expect("wire").encode();
        good.push(b'x');
        assert!(matches!(
            WireEnvelope::decode(&good),
            Err(MessengerError::Transport { message }) if message.contains("length mismatch")
        ));
    }

    #[test]
    fn decode_rejects_non_utf8_name() {
        let mut bytes = b"SERW".to_vec();
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.push(0xff);
        bytes.extend_from_slice(&0_u32.to_be_bytes());
        assert!(matches!(
            WireEnvelope::decode(&bytes),
            Err(MessengerError::Transport { message }) if message.contains("utf-8")
        ));
    }

    #[test]
    fn decode_rejects_claimed_huge_name() {
        let mut bytes = b"SERW".to_vec();
        bytes.extend_from_slice(&u32::MAX.to_be_bytes());
        assert!(matches!(
            WireEnvelope::decode(&bytes),
            Err(MessengerError::Transport { .. })
        ));
    }
}
