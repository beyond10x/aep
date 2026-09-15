//! Canonical byte framing and SHA-256 values for raw planning migration captures.

use std::fmt;

use schemars::schema::{InstanceType, Schema, SchemaObject};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest as _, Sha256};

/// The raw-capture format scalar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawCaptureFormatV1;

impl RawCaptureFormatV1 {
    /// The only admitted wire spelling.
    pub const VALUE: &'static str = "aep.raw-capture/1";
}

impl Serialize for RawCaptureFormatV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(Self::VALUE)
    }
}

impl<'de> Deserialize<'de> for RawCaptureFormatV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value == Self::VALUE {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "expected {}, found {value:?}",
                Self::VALUE
            )))
        }
    }
}

impl schemars::JsonSchema for RawCaptureFormatV1 {
    fn schema_name() -> String {
        "RawCaptureFormatV1".to_owned()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            enum_values: Some(vec![serde_json::Value::String(Self::VALUE.to_owned())]),
            ..SchemaObject::default()
        };
        schema.metadata().description = Some("The scalar literal aep.raw-capture/1.".to_owned());
        schema.into()
    }
}

/// Exact bytes written as `hex:` followed by canonical lowercase hexadecimal pairs.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HexBytesV1(Vec<u8>);

impl HexBytesV1 {
    /// Wraps exact bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Returns the exact bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the value into its exact bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Parses the canonical wire spelling.
    pub fn parse(value: &str) -> Result<Self, ScalarParseError> {
        let digits = value
            .strip_prefix("hex:")
            .ok_or(ScalarParseError::HexPrefix)?;
        decode_lower_hex(digits)
            .map(Self)
            .map_err(ScalarParseError::HexDigits)
    }

    /// Returns the canonical wire spelling.
    pub fn as_wire(&self) -> String {
        format!("hex:{}", encode_lower_hex(&self.0))
    }
}

impl fmt::Display for HexBytesV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_wire())
    }
}

impl Serialize for HexBytesV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_wire())
    }
}

impl<'de> Deserialize<'de> for HexBytesV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for HexBytesV1 {
    fn schema_name() -> String {
        "HexBytesV1".to_owned()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> Schema {
        string_pattern_schema(
            "^hex:(?:[0-9a-f]{2})*$",
            "Exact bytes encoded as lowercase hexadecimal pairs after hex:.",
        )
    }
}

/// A canonical SHA-256 value written as `sha256:` plus 64 lowercase hexadecimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DigestV1([u8; 32]);

impl DigestV1 {
    /// Builds a digest from its decoded bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the decoded 32-byte digest.
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    /// Borrows the decoded 32-byte digest.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Parses the canonical wire spelling.
    pub fn parse(value: &str) -> Result<Self, ScalarParseError> {
        let digits = value
            .strip_prefix("sha256:")
            .ok_or(ScalarParseError::DigestPrefix)?;
        if digits.len() != 64 {
            return Err(ScalarParseError::DigestLength(digits.len()));
        }
        let decoded = decode_lower_hex(digits).map_err(ScalarParseError::DigestDigits)?;
        let mut bytes = [0_u8; 32];
        bytes.copy_from_slice(&decoded);
        Ok(Self(bytes))
    }

    /// Returns the canonical wire spelling.
    pub fn as_wire(&self) -> String {
        format!("sha256:{}", encode_lower_hex(&self.0))
    }
}

impl fmt::Display for DigestV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_wire())
    }
}

impl Serialize for DigestV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.as_wire())
    }
}

impl<'de> Deserialize<'de> for DigestV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for DigestV1 {
    fn schema_name() -> String {
        "DigestV1".to_owned()
    }

    fn json_schema(_: &mut schemars::gen::SchemaGenerator) -> Schema {
        string_pattern_schema(
            "^sha256:[0-9a-f]{64}$",
            "A canonical SHA-256 digest with its algorithm prefix.",
        )
    }
}

/// A canonical scalar parse failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScalarParseError {
    /// The byte string omitted its `hex:` prefix.
    #[error("byte string must start with hex:")]
    HexPrefix,
    /// The digest omitted its `sha256:` prefix.
    #[error("digest must start with sha256:")]
    DigestPrefix,
    /// The digest had the wrong number of hexadecimal digits.
    #[error("digest must contain 64 hexadecimal digits, found {0}")]
    DigestLength(usize),
    /// The byte string's hexadecimal body was not canonical.
    #[error("invalid byte-string hex: {0}")]
    HexDigits(HexParseError),
    /// The digest's hexadecimal body was not canonical.
    #[error("invalid digest hex: {0}")]
    DigestDigits(HexParseError),
}

/// A canonical hexadecimal parse failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HexParseError {
    /// Hexadecimal digits must arrive in complete byte pairs.
    #[error("hexadecimal digit count must be even, found {0}")]
    OddLength(usize),
    /// Only lowercase ASCII hexadecimal digits are admitted.
    #[error("non-lowercase-hexadecimal digit at byte {0}")]
    InvalidDigit(usize),
}

/// A checked canonical framing failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// The platform could not represent the part count as a wire `u64`.
    #[error("part count does not fit u64")]
    PartCount,
    /// The platform could not represent one part length as a wire `u64`.
    #[error("part length does not fit u64")]
    PartLength,
}

/// A value that emits the raw-capture transcript's ordered flat part list.
pub trait CanonicalPartsV1 {
    /// Appends the value's parts without framing them.
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>);

    /// Returns the complete ordered flat part list.
    fn canonical_parts(&self) -> Vec<Vec<u8>> {
        let mut parts = Vec::new();
        self.append_parts(&mut parts);
        parts
    }
}

impl CanonicalPartsV1 for RawCaptureFormatV1 {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        parts.push(Self::VALUE.as_bytes().to_vec());
    }
}

impl CanonicalPartsV1 for HexBytesV1 {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        parts.push(self.0.clone());
    }
}

impl CanonicalPartsV1 for DigestV1 {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        parts.push(self.0.to_vec());
    }
}

impl CanonicalPartsV1 for String {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        parts.push(self.as_bytes().to_vec());
    }
}

impl CanonicalPartsV1 for str {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        parts.push(self.as_bytes().to_vec());
    }
}

impl CanonicalPartsV1 for bool {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        parts.push(vec![u8::from(*self)]);
    }
}

macro_rules! fixed_integer_parts {
    ($($type:ty),* $(,)?) => {
        $(
            impl CanonicalPartsV1 for $type {
                fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
                    parts.push(self.to_be_bytes().to_vec());
                }
            }
        )*
    };
}

fixed_integer_parts!(u16, u32, u64, i64);

impl<T: CanonicalPartsV1> CanonicalPartsV1 for Vec<T> {
    fn append_parts(&self, parts: &mut Vec<Vec<u8>>) {
        let count = u64::try_from(self.len()).expect("Vec length fits the raw-capture u64 grammar");
        parts.push(count.to_be_bytes().to_vec());
        for item in self {
            item.append_parts(parts);
        }
    }
}

/// Frames an ordered flat part list as `domain || 00 || count || lengths-and-parts`.
pub fn frame_v1(domain: &str, parts: &[Vec<u8>]) -> Result<Vec<u8>, FrameError> {
    let count = u64::try_from(parts.len()).map_err(|_| FrameError::PartCount)?;
    let capacity = domain
        .len()
        .saturating_add(1)
        .saturating_add(8)
        .saturating_add(
            parts
                .iter()
                .map(|part| 8_usize.saturating_add(part.len()))
                .sum(),
        );
    let mut framed = Vec::with_capacity(capacity);
    framed.extend_from_slice(domain.as_bytes());
    framed.push(0);
    framed.extend_from_slice(&count.to_be_bytes());
    for part in parts {
        let length = u64::try_from(part.len()).map_err(|_| FrameError::PartLength)?;
        framed.extend_from_slice(&length.to_be_bytes());
        framed.extend_from_slice(part);
    }
    Ok(framed)
}

/// Computes SHA-256 over a canonical frame.
pub fn digest_parts_v1(domain: &str, parts: &[Vec<u8>]) -> Result<DigestV1, FrameError> {
    let digest: [u8; 32] = Sha256::digest(frame_v1(domain, parts)?).into();
    Ok(DigestV1::from_bytes(digest))
}

/// Computes a domain-separated SHA-256 digest over one exact byte string.
pub fn digest_bytes_v1(domain: &str, bytes: &[u8]) -> DigestV1 {
    digest_parts_v1(domain, &[bytes.to_vec()]).expect("one in-memory byte slice fits the grammar")
}

/// Binds the exact selected project-file bytes.
pub fn selector_digest_v1(bytes: &[u8]) -> DigestV1 {
    digest_bytes_v1("aep.migration.project-selector/1", bytes)
}

/// Binds ephemeral exact effective private configuration bytes without retaining them.
pub fn config_digest_v1(bytes: &[u8]) -> DigestV1 {
    digest_bytes_v1("aep.migration.source-config/1", bytes)
}

/// Computes the canonical ordering key for a raw-capture value.
pub fn sort_key_v1<T: CanonicalPartsV1>(value: &T) -> Vec<u8> {
    frame_v1("aep.migration.raw-sort-key/1", &value.canonical_parts())
        .expect("an in-memory value fits the raw-capture framing grammar")
}

fn decode_lower_hex(digits: &str) -> Result<Vec<u8>, HexParseError> {
    if digits.len() % 2 != 0 {
        return Err(HexParseError::OddLength(digits.len()));
    }
    let bytes = digits.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for offset in (0..bytes.len()).step_by(2) {
        let high = lower_hex_value(bytes[offset]).ok_or(HexParseError::InvalidDigit(offset))?;
        let low =
            lower_hex_value(bytes[offset + 1]).ok_or(HexParseError::InvalidDigit(offset + 1))?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

fn lower_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn encode_lower_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn string_pattern_schema(pattern: &str, description: &str) -> Schema {
    let mut schema = SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        ..SchemaObject::default()
    };
    schema.string().pattern = Some(pattern.to_owned());
    schema.metadata().description = Some(description.to_owned());
    schema.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_frame_and_digest_match_the_literal_empty_vector() {
        const EXPECTED: &[u8] = &[
            0x61, 0x65, 0x70, 0x2e, 0x6d, 0x69, 0x67, 0x72, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x2e,
            0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00,
        ];
        let framed = frame_v1("aep.migration.test/1", &[]).expect("frames");
        assert_eq!(framed, EXPECTED);
        assert_eq!(
            digest_parts_v1("aep.migration.test/1", &[])
                .expect("hashes")
                .as_wire(),
            "sha256:355279afe5bea97f0fec16a7f42e45c268a0dd0c7e8b6db35d3295cbc9af27ed"
        );
    }

    #[test]
    fn canonical_frame_and_digest_match_the_literal_two_part_vector() {
        const EXPECTED: &[u8] = &[
            0x61, 0x65, 0x70, 0x2e, 0x6d, 0x69, 0x67, 0x72, 0x61, 0x74, 0x69, 0x6f, 0x6e, 0x2e,
            0x74, 0x65, 0x73, 0x74, 0x2f, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x02, 0x00, 0xff,
        ];
        let parts = vec![Vec::new(), vec![0x00, 0xff]];
        let framed = frame_v1("aep.migration.test/1", &parts).expect("frames");
        assert_eq!(framed, EXPECTED);
        assert_eq!(
            digest_parts_v1("aep.migration.test/1", &parts)
                .expect("hashes")
                .as_wire(),
            "sha256:381a16f9663dd619850ee3f3e2c78f3bf95f309477e6164ac0a3d112632371cd"
        );
    }

    #[test]
    fn scalar_readers_admit_only_canonical_spellings() {
        assert_eq!(
            HexBytesV1::parse("hex:").expect("empty bytes").as_bytes(),
            &[] as &[u8]
        );
        assert_eq!(
            HexBytesV1::parse("hex:00ff").expect("bytes").as_bytes(),
            &[0x00, 0xff]
        );
        for invalid in ["", "00", "hex:0", "hex:0F", "hex:gg"] {
            assert!(HexBytesV1::parse(invalid).is_err(), "{invalid:?}");
        }
        let valid = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(DigestV1::parse(valid).expect("digest").as_wire(), valid);
        for invalid in [
            "0000000000000000000000000000000000000000000000000000000000000000",
            "sha256:00",
            "sha256:000000000000000000000000000000000000000000000000000000000000000G",
        ] {
            assert!(DigestV1::parse(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn raw_capture_format_is_one_scalar_literal() {
        assert_eq!(
            serde_json::to_string(&RawCaptureFormatV1).expect("serialises"),
            "\"aep.raw-capture/1\""
        );
        assert!(serde_json::from_str::<RawCaptureFormatV1>("\"aep.raw-capture/2\"").is_err());
    }
}
