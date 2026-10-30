use rustc_dep_info::{Checksum, ChecksumAlgo, InvalidChecksum};

const SHA256_ABC: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const BLAKE3_ABC: &str = "blake3=6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

#[test]
fn rejects_unsupported_algorithm() {
    assert!(matches!(
        "md5".parse::<ChecksumAlgo>(),
        Err(InvalidChecksum::InvalidChecksumAlgo)
    ));
}

#[test]
fn rejects_missing_digest() {
    assert!(matches!(
        "sha256".parse::<Checksum>(),
        Err(InvalidChecksum::InvalidFormat)
    ));
}

#[test]
fn rejects_empty_digest() {
    assert!(matches!(
        "sha256=".parse::<Checksum>(),
        Err(InvalidChecksum::InvalidChecksum(ChecksumAlgo::Sha256))
    ));
}

#[test]
fn rejects_short_digest() {
    assert!(matches!(
        "sha256=00".parse::<Checksum>(),
        Err(InvalidChecksum::InvalidChecksum(ChecksumAlgo::Sha256))
    ));
}

#[test]
fn rejects_non_hexadecimal_digest() {
    let input = format!("sha256={}", "z".repeat(64));
    assert!(matches!(
        input.parse::<Checksum>(),
        Err(InvalidChecksum::InvalidChecksum(ChecksumAlgo::Sha256))
    ));
}

#[test]
fn rejects_long_digest() {
    let input = format!("sha256={}", "0".repeat(66));
    assert!(matches!(
        input.parse::<Checksum>(),
        Err(InvalidChecksum::InvalidChecksum(ChecksumAlgo::Sha256))
    ));
}

#[test]
fn normalizes_hex_case_and_preserves_upstreams_extra_field_handling() {
    let expected: Checksum = SHA256_ABC.parse().unwrap();
    let upper_hex = format!(
        "sha256={}",
        SHA256_ABC.split_once('=').unwrap().1.to_uppercase()
    );
    assert_eq!(upper_hex.parse::<Checksum>().unwrap(), expected);
    assert_eq!(
        format!("{SHA256_ABC}=extra").parse::<Checksum>().unwrap(),
        expected
    );
    assert_eq!(Checksum::new(expected.algo(), *expected.value()), expected);
}

#[test]
fn serde_uses_the_upstream_sha256_string_representation() {
    let checksum: Checksum = SHA256_ABC.parse().unwrap();
    let json = serde_json::to_string(&checksum).unwrap();
    assert_eq!(json, format!("\"{SHA256_ABC}\""));
    assert_eq!(serde_json::from_str::<Checksum>(&json).unwrap(), checksum);
}

#[test]
fn serde_uses_the_upstream_blake3_string_representation() {
    let checksum: Checksum = BLAKE3_ABC.parse().unwrap();
    let json = serde_json::to_string(&checksum).unwrap();
    assert_eq!(json, format!("\"{BLAKE3_ABC}\""));
    assert_eq!(serde_json::from_str::<Checksum>(&json).unwrap(), checksum);
}

#[test]
fn serde_rejects_invalid_checksum() {
    assert!(serde_json::from_str::<Checksum>(r#""sha256=invalid""#).is_err());
}
