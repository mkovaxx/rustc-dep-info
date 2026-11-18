use rustc_dep_info::{Contents, Digest, ParseError, ParseErrorKind};

const SHA256_ABC: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn parse_checksum(text: &str) -> Result<Digest, ParseError> {
    let input = format!(
        "a:
# checksum:{text} file_len:3 a"
    );
    let contents: Contents = input.parse()?;
    Ok(contents
        .sources
        .into_values()
        .next()
        .unwrap()
        .unwrap()
        .digest)
}

#[test]
fn rejects_invalid_checksum_records() {
    for (input, column, expected) in [
        (
            "md5=00".to_owned(),
            12,
            ParseErrorKind::InvalidChksumAlgorithm,
        ),
        ("sha256".to_owned(), 12, ParseErrorKind::InvalidChksumFormat),
        (
            "sha256=".to_owned(),
            19,
            ParseErrorKind::InvalidChksumDigest,
        ),
        (
            "sha256=00".to_owned(),
            19,
            ParseErrorKind::InvalidChksumDigest,
        ),
        (
            "blake3=00".to_owned(),
            19,
            ParseErrorKind::InvalidChksumDigest,
        ),
        (
            format!("sha256={}", "z".repeat(64)),
            19,
            ParseErrorKind::InvalidChksumDigest,
        ),
        (
            format!("sha256={}", "0".repeat(66)),
            19,
            ParseErrorKind::InvalidChksumDigest,
        ),
        (
            format!("{SHA256_ABC}=extra"),
            12,
            ParseErrorKind::InvalidChksumFormat,
        ),
    ] {
        let error = parse_checksum(&input).unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (2, column, expected),
            "{input}"
        );
    }
}

#[test]
fn normalizes_hex_case() {
    let expected = parse_checksum(SHA256_ABC).unwrap();
    let upper_hex = format!(
        "sha256={}",
        SHA256_ABC.split_once('=').unwrap().1.to_uppercase()
    );
    assert_eq!(parse_checksum(&upper_hex).unwrap(), expected);
    assert_eq!(expected.to_string(), SHA256_ABC);
}

#[test]
fn constructs_and_formats_both_variants() {
    for checksum in [Digest::Sha256([0x12; 32]), Digest::Blake3([0x34; 32])] {
        assert_eq!(parse_checksum(&checksum.to_string()).unwrap(), checksum);
    }
}
