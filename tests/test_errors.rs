use rustc_dep_info::{Contents, ParseErrorKind::*};

const SHA256: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn rejects_invalid_paths_at_original_character_columns() {
    for (text, line, column, kind) in [
        (": a", 1, 1, EmptyTarget),
        (":", 1, 1, EmptySource),
        (r"src\:", 1, 4, SourceEndsWithBackslash),
        (r"app: src/main.rs\", 1, 17, SourceEndsWithBackslash),
        ("bad\rpath:", 1, 4, SourceContainsCarriageReturn),
        ("bad\rtarget: a", 1, 4, TargetContainsCarriageReturn),
        ("é: a  b", 1, 6, EmptySource),
        ("é: a ", 1, 6, EmptySource),
        (r"é: a\ b\", 1, 8, SourceEndsWithBackslash),
        ("é: a\\ b\r:", 1, 8, SourceContainsCarriageReturn),
        ("a\\ b\r:", 1, 5, SourceContainsCarriageReturn),
        ("# ordinary comment", 1, 1, UnknownSyntax),
        ("ordinary text", 1, 1, UnknownSyntax),
    ] {
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (line, column, kind),
            "{text:?}"
        );
    }
}

#[test]
fn rejects_inconsistent_and_duplicate_rules() {
    for (text, line, column, kind) in [
        ("app: a\napp: a\na:", 2, 1, DuplicateTarget),
        ("app: a a", 1, 8, DuplicateSource),
        ("é: a a", 1, 6, DuplicateSource),
        (r"é: a\ b a\ b", 1, 9, DuplicateSource),
        ("app: a\na:\na:", 3, 1, DuplicateSourceRule),
        ("app: a\nother: b\na:", 2, 8, TargetSourcesMismatch),
        // Dependency lists must have identical lengths and ordering.
        ("app: a b\nother: a", 2, 1, TargetSourcesMismatch),
        ("app: a\nother: a b", 2, 10, TargetSourcesMismatch),
        ("app: a b\nother: b a", 2, 8, TargetSourcesMismatch),
        ("app: a\nother: ", 2, 1, TargetSourcesMismatch),
        ("app: \nother: a", 2, 8, TargetSourcesMismatch),
        ("é: a\na:\nb:", 3, 1, UnexpectedSourceRule),
        ("app: a", 1, 6, MissingSourceRule),
        ("app: a\n", 1, 6, MissingSourceRule),
        ("é: a b\na:", 1, 6, MissingSourceRule),
        // Missing rules are reported in dependency order, with original columns.
        ("app: b a", 1, 6, MissingSourceRule),
        ("app: b a\na:", 1, 6, MissingSourceRule),
        // Local duplicates take precedence over cross-rule inconsistencies.
        ("app: a\nother: a a", 2, 10, DuplicateSource),
        ("app: a\napp: b", 2, 1, DuplicateTarget),
        ("app: a\napp: b b", 2, 8, DuplicateSource),
    ] {
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (line, column, kind),
            "{text:?}"
        );
    }
}

#[test]
fn rejects_invalid_and_duplicate_environment_dependencies() {
    for (text, line, column, kind) in [
        (r"# env-dep:BAD\t=value", 1, 14, InvalidEnvEscape),
        (r"# env-dep:NAME=value\t", 1, 21, InvalidEnvEscape),
        (r"# env-dep:NAME=value\", 1, 21, UnterminatedEnvEscape),
        (r"# env-dep:NAME\", 1, 15, UnterminatedEnvEscape),
        (r"# env-dep:é\=clé=ok\n\t", 1, 22, InvalidEnvEscape),
        (r"# env-dep:é\=clé=ok\n\", 1, 22, UnterminatedEnvEscape),
        (
            "# env-dep:MODE=debug\n# env-dep:MODE=release",
            2,
            11,
            DuplicateEnvKey,
        ),
        ("# env-dep:UNSET\n# env-dep:UNSET", 2, 11, DuplicateEnvKey),
        ("# env-dep:UNSET\n# env-dep:UNSET=", 2, 11, DuplicateEnvKey),
        ("# env-dep:SET=\n# env-dep:SET", 2, 11, DuplicateEnvKey),
        // Duplicate detection applies to decoded keys containing escaped equals signs.
        (
            "# env-dep:A\\=B=1\n# env-dep:A\\=B=2",
            2,
            11,
            DuplicateEnvKey,
        ),
        ("# env-dep:é=1\n# env-dep:é=2", 2, 11, DuplicateEnvKey),
        // These distinct spellings decode to the same key.
        (
            "# env-dep:A\\rB=1\n# env-dep:A\rB=2",
            2,
            11,
            DuplicateEnvKey,
        ),
        // A valid escaped backslash must not consume the assignment delimiter.
        (
            "# env-dep:A\\\\=1\n# env-dep:A\\\\=2",
            2,
            11,
            DuplicateEnvKey,
        ),
    ] {
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (line, column, kind),
            "{text:?}"
        );
    }
}

#[test]
fn rejects_all_backward_section_transitions() {
    let checksum = format!("# checksum:{SHA256} file_len:3 a");
    // Each prefix is valid on its own; the final record belongs to an earlier section.
    for (prefix, record, line) in [
        ("app: a\na:".to_owned(), "other: a", 3),
        ("app: a\na:\n# env-dep:K=V".to_owned(), "other: a", 4),
        ("app: a\na:\n# env-dep:K=V".to_owned(), "a:", 4),
        (format!("app: a\na:\n{checksum}"), "other: a", 4),
        (format!("app: a\na:\n{checksum}"), "a:", 4),
        (format!("app: a\na:\n{checksum}"), "# env-dep:K=V", 4),
    ] {
        assert!(prefix.parse::<Contents>().is_ok(), "{prefix:?}");
        let text = format!("{prefix}\n{record}");
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (line, 1, LineInWrongSection),
            "{text:?}"
        );
    }
}

#[test]
fn rejects_invalid_checksum_fields_and_references() {
    let prefix = format!("# checksum:{SHA256}");
    let length_column = prefix.chars().count() + 2;
    let path_column = length_column + "file_len:3 ".len();
    for (text, line, column, kind) in [
        (
            "app: src/main.rs\nsrc/main.rs:\n# checksum:sha256=invalid file_len:3 src/main.rs"
                .to_owned(),
            3,
            19,
            InvalidChksumDigest,
        ),
        (
            prefix.clone(),
            1,
            length_column - 1,
            InvalidChksumFileLength,
        ),
        (
            format!("{prefix} "),
            1,
            length_column,
            InvalidChksumFileLength,
        ),
        (
            format!("{prefix} length:3 a"),
            1,
            length_column,
            InvalidChksumFileLength,
        ),
        (
            format!("{prefix} file_len:invalid a"),
            1,
            length_column,
            InvalidChksumFileLength,
        ),
        (
            format!("{prefix} file_len:-1 a"),
            1,
            length_column,
            InvalidChksumFileLength,
        ),
        (
            format!("{prefix} file_len:18446744073709551616 a"),
            1,
            length_column,
            InvalidChksumFileLength,
        ),
        (
            format!("{prefix} file_len:3"),
            1,
            path_column - 1,
            InvalidChksumSyntax,
        ),
        (format!("{prefix} file_len:3 "), 1, path_column, EmptySource),
        (
            format!("{prefix} file_len:3 é\\ b\r"),
            1,
            path_column + 4,
            SourceContainsCarriageReturn,
        ),
        (
            format!("{prefix} file_len:3 a\\"),
            1,
            path_column + 1,
            SourceEndsWithBackslash,
        ),
        (
            format!("{prefix} file_len:3 é"),
            1,
            path_column,
            UnknownChksumSource,
        ),
        (
            format!("app: b\nb:\n{prefix} file_len:3 a"),
            3,
            path_column,
            UnknownChksumSource,
        ),
        (
            format!("app: a\na:\n{prefix} file_len:3 a\n{prefix} file_len:3 a"),
            4,
            path_column,
            DuplicateChksum,
        ),
        (
            format!("é:\n{prefix} file_len:3 é\n{prefix} file_len:3 é"),
            3,
            path_column,
            DuplicateChksum,
        ),
    ] {
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (line, column, kind),
            "{text:?}"
        );
    }
}

#[test]
fn rejects_invalid_checksum_records() {
    for (input, column, expected) in [
        ("md5=00".to_owned(), 12, InvalidChksumAlgorithm),
        ("sha256".to_owned(), 12, InvalidChksumFormat),
        ("sha256=".to_owned(), 19, InvalidChksumDigest),
        ("sha256=00".to_owned(), 19, InvalidChksumDigest),
        ("blake3=00".to_owned(), 19, InvalidChksumDigest),
        (
            format!("sha256={}", "z".repeat(64)),
            19,
            InvalidChksumDigest,
        ),
        (
            format!("sha256={}", "0".repeat(66)),
            19,
            InvalidChksumDigest,
        ),
        (format!("{SHA256}=extra"), 12, InvalidChksumFormat),
    ] {
        let text = format!("a:\n# checksum:{input} file_len:3 a");
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (2, column, expected),
            "{input}"
        );
    }
}
