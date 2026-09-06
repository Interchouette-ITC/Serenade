use std::any::{Any, TypeId};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    deserialize_value, serde_supports_format, serialize_value, version, Decoder, Denormalizer,
    Encoder, JsonDecoder, JsonEncoder, NormalizationContext, Normalizer, NormalizerRegistry,
    Serializer, SerializerError, FORMAT_JSON,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProductDto {
    sku: String,
    price_cents: u64,
}

struct Tag(String);

struct TagCodec;

impl Normalizer for TagCodec {
    fn supports_normalization(&self, object: &dyn Any, format: &str) -> bool {
        format.eq_ignore_ascii_case(FORMAT_JSON) && object.is::<Tag>()
    }

    fn normalize(
        &self,
        object: &dyn Any,
        _format: &str,
        _context: &NormalizationContext,
    ) -> Result<Value, SerializerError> {
        let tag = object
            .downcast_ref::<Tag>()
            .ok_or_else(|| SerializerError::Normalization {
                message: "expected Tag".to_owned(),
            })?;
        Ok(json!({ "label": tag.0 }))
    }
}

impl Denormalizer for TagCodec {
    fn supports_denormalization(&self, type_id: TypeId, format: &str) -> bool {
        format.eq_ignore_ascii_case(FORMAT_JSON) && type_id == TypeId::of::<Tag>()
    }

    fn denormalize(
        &self,
        data: &Value,
        _type_id: TypeId,
        _format: &str,
        _context: &NormalizationContext,
    ) -> Result<Box<dyn Any + Send + Sync>, SerializerError> {
        let label = data.get("label").and_then(Value::as_str).ok_or_else(|| {
            SerializerError::Normalization {
                message: "missing label".to_owned(),
            }
        })?;
        Ok(Box::new(Tag(label.to_owned())))
    }
}

#[test]
fn version_is_semverish() {
    assert_ne!(version(), "");
}

#[test]
fn serde_bridge_json_roundtrip() {
    let dto = ProductDto {
        sku: "SKU-1".to_owned(),
        price_cents: 1999,
    };
    let bytes = serialize_value(&dto, FORMAT_JSON).expect("serialize");
    let back: ProductDto = deserialize_value(&bytes, FORMAT_JSON).expect("deserialize");
    assert_eq!(back, dto);
    assert!(serde_supports_format("JSON"));
    assert!(!serde_supports_format("xml"));
}

#[test]
fn serde_bridge_rejects_unknown_format() {
    let err = serialize_value(&1_u32, "xml").expect_err("xml");
    assert!(matches!(err, SerializerError::UnsupportedFormat { .. }));
}

#[test]
fn json_encoder_decoder_roundtrip() {
    let value = json!({"a": 1});
    let bytes = JsonEncoder.encode(&value, FORMAT_JSON).expect("encode");
    let back = JsonDecoder.decode(&bytes, FORMAT_JSON).expect("decode");
    assert_eq!(back, value);
}

#[test]
fn json_codec_rejects_other_format() {
    let err = JsonEncoder.encode(&json!(true), "yaml").expect_err("yaml");
    assert!(matches!(err, SerializerError::UnsupportedFormat { .. }));
    let err = JsonDecoder.decode(b"{}", "yaml").expect_err("yaml");
    assert!(matches!(err, SerializerError::UnsupportedFormat { .. }));
}

#[test]
fn registry_custom_normalizer_roundtrip() {
    let mut registry = NormalizerRegistry::new();
    registry.add_normalizer(TagCodec);
    registry.add_denormalizer(TagCodec);
    assert_eq!(registry.normalizer_count(), 1);
    assert_eq!(registry.denormalizer_count(), 1);

    let ctx = NormalizationContext::new();
    let value = registry
        .normalize(&Tag("alpha".to_owned()), FORMAT_JSON, &ctx)
        .expect("normalize");
    assert_eq!(value, json!({"label": "alpha"}));

    let boxed = registry
        .denormalize(&value, TypeId::of::<Tag>(), FORMAT_JSON, &ctx)
        .expect("denormalize");
    assert_eq!(boxed.downcast_ref::<Tag>().expect("tag").0, "alpha");
}

#[test]
fn registry_unsupported_type() {
    let registry = NormalizerRegistry::new();
    let err = registry
        .normalize(
            &Tag("x".to_owned()),
            FORMAT_JSON,
            &NormalizationContext::new(),
        )
        .expect_err("missing");
    assert!(matches!(err, SerializerError::UnsupportedType { .. }));
}

#[test]
fn serializer_facade_with_custom_codec() {
    let mut serializer = Serializer::new();
    serializer.registry_mut().add_normalizer(TagCodec);
    serializer.registry_mut().add_denormalizer(TagCodec);

    let bytes = serializer
        .serialize(&Tag("beta".to_owned()), FORMAT_JSON)
        .expect("serialize");
    let boxed = serializer
        .deserialize(&bytes, TypeId::of::<Tag>(), FORMAT_JSON)
        .expect("deserialize");
    assert_eq!(boxed.downcast_ref::<Tag>().expect("tag").0, "beta");
}

#[test]
fn serializer_rejects_unknown_format_after_normalize() {
    let mut serializer = Serializer::new();
    serializer.registry_mut().add_normalizer(TagCodec);
    let err = serializer
        .serialize(&Tag("z".to_owned()), "xml")
        .expect_err("xml");
    // TagCodec only supports json, so this fails at normalize (unsupported type for format).
    assert!(matches!(
        err,
        SerializerError::UnsupportedType { .. } | SerializerError::UnsupportedFormat { .. }
    ));
}

#[test]
fn normalization_context_attributes() {
    let mut ctx = NormalizationContext::new();
    ctx.set("groups", "api");
    assert_eq!(ctx.get("groups"), Some("api"));
    assert_eq!(ctx.get("missing"), None);
}

#[test]
fn deserialize_value_codec_error_on_bad_json() {
    let err = deserialize_value::<ProductDto>(b"not-json", FORMAT_JSON).expect_err("bad");
    assert!(matches!(err, SerializerError::Codec { .. }));
}

#[test]
fn deserialize_value_rejects_type_mismatch() {
    let err = deserialize_value::<ProductDto>(br#"{"sku":1}"#, FORMAT_JSON).expect_err("type");
    assert!(matches!(err, SerializerError::Codec { .. }));
}

#[test]
fn deserialize_value_rejects_unknown_format() {
    let err = deserialize_value::<u32>(b"1", "xml").expect_err("xml");
    assert!(matches!(err, SerializerError::UnsupportedFormat { .. }));
}

struct AnyFormatNormalizer;

impl Normalizer for AnyFormatNormalizer {
    fn supports_normalization(&self, object: &dyn Any, _format: &str) -> bool {
        object.is::<Tag>()
    }

    fn normalize(
        &self,
        object: &dyn Any,
        _format: &str,
        _context: &NormalizationContext,
    ) -> Result<Value, SerializerError> {
        let tag = object
            .downcast_ref::<Tag>()
            .ok_or_else(|| SerializerError::Normalization {
                message: "expected Tag".to_owned(),
            })?;
        Ok(json!({ "label": tag.0 }))
    }
}

struct NeverEncoder;

impl Encoder for NeverEncoder {
    fn supports(&self, _format: &str) -> bool {
        false
    }

    fn encode(&self, _data: &Value, format: &str) -> Result<Vec<u8>, SerializerError> {
        Err(SerializerError::UnsupportedFormat {
            format: format.to_owned(),
        })
    }
}

struct NeverDecoder;

impl Decoder for NeverDecoder {
    fn supports(&self, _format: &str) -> bool {
        false
    }

    fn decode(&self, _data: &[u8], format: &str) -> Result<Value, SerializerError> {
        Err(SerializerError::UnsupportedFormat {
            format: format.to_owned(),
        })
    }
}

#[test]
fn serializer_default_and_registry_accessors() {
    let mut serializer = Serializer::default();
    assert_eq!(serializer.registry().normalizer_count(), 0);
    serializer.registry_mut().add_normalizer(TagCodec);
    assert_eq!(serializer.registry().normalizer_count(), 1);
}

#[test]
fn serializer_with_context_and_custom_codecs() {
    let mut serializer = Serializer::new();
    serializer.registry_mut().add_normalizer(TagCodec);
    serializer.registry_mut().add_denormalizer(TagCodec);
    serializer.add_encoder(NeverEncoder);
    serializer.add_decoder(NeverDecoder);

    let mut ctx = NormalizationContext::new();
    ctx.set("groups", "api");
    let bytes = serializer
        .serialize_with_context(&Tag("gamma".to_owned()), FORMAT_JSON, &ctx)
        .expect("serialize");
    let boxed = serializer
        .deserialize_with_context(&bytes, TypeId::of::<Tag>(), FORMAT_JSON, &ctx)
        .expect("deserialize");
    assert_eq!(boxed.downcast_ref::<Tag>().expect("tag").0, "gamma");
}

#[test]
fn serializer_encode_unsupported_when_no_encoder_matches() {
    let mut serializer = Serializer::new();
    serializer
        .registry_mut()
        .add_normalizer(AnyFormatNormalizer);
    // Replace default JSON encoder path by only registering a never-matching encoder after
    // clearing is impossible; instead normalize for xml then fail encode.
    let err = serializer
        .serialize(&Tag("z".to_owned()), "xml")
        .expect_err("xml");
    assert!(matches!(err, SerializerError::UnsupportedFormat { .. }));
}

#[test]
fn serializer_decode_unsupported_format() {
    let mut serializer = Serializer::new();
    serializer.registry_mut().add_denormalizer(TagCodec);
    let err = serializer
        .deserialize(b"{}", TypeId::of::<Tag>(), "xml")
        .expect_err("xml");
    assert!(matches!(err, SerializerError::UnsupportedFormat { .. }));
}

#[test]
fn registry_denormalize_unsupported() {
    let registry = NormalizerRegistry::new();
    let err = registry
        .denormalize(
            &json!({}),
            TypeId::of::<Tag>(),
            FORMAT_JSON,
            &NormalizationContext::new(),
        )
        .expect_err("missing");
    assert!(matches!(err, SerializerError::UnsupportedType { .. }));
}

#[test]
fn tag_codec_normalization_error_paths() {
    let codec = TagCodec;
    let ctx = NormalizationContext::new();
    let err = codec
        .normalize(&42_u32, FORMAT_JSON, &ctx)
        .expect_err("wrong type");
    assert!(matches!(err, SerializerError::Normalization { .. }));
    let err = codec
        .denormalize(&json!({}), TypeId::of::<Tag>(), FORMAT_JSON, &ctx)
        .expect_err("missing label");
    assert!(matches!(err, SerializerError::Normalization { .. }));
}
