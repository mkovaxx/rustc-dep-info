use rustc_dep_info::{Checksum, ChecksumAlgo, Contents, ParseError};

const SHA256_ABC: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn parse_checksum(text: &str) -> Result<Checksum, ParseError> {
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
        .checksum)
}

#[test]
fn rejects_invalid_checksum_records() {
    for (input, expected) in [
        ("md5=00".to_owned(), ParseError::InvalidChecksumAlgorithm),
        ("sha256".to_owned(), ParseError::InvalidChecksumFormat),
        (
            "sha256=".to_owned(),
            ParseError::InvalidChecksumDigest(ChecksumAlgo::Sha256),
        ),
        (
            "sha256=00".to_owned(),
            ParseError::InvalidChecksumDigest(ChecksumAlgo::Sha256),
        ),
        (
            format!("sha256={}", "z".repeat(64)),
            ParseError::InvalidChecksumDigest(ChecksumAlgo::Sha256),
        ),
        (
            format!("sha256={}", "0".repeat(66)),
            ParseError::InvalidChecksumDigest(ChecksumAlgo::Sha256),
        ),
        (
            format!("{SHA256_ABC}=extra"),
            ParseError::InvalidChecksumFormat,
        ),
    ] {
        assert_eq!(parse_checksum(&input), Err(expected), "{input}");
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
