use indexmap::{IndexMap, IndexSet};

use rustc_dep_info::{
    Contents, Digest, EnvKey, EnvVal, Metadata, ParseError, ParseErrorKind, Source, Target,
};

const SHA256: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn source(path: &str) -> Source {
    let text = format!("{}:", path.replace(' ', "\\ "));
    text.parse::<Contents>()
        .unwrap()
        .sources
        .into_keys()
        .next()
        .unwrap()
}

fn target(path: &str) -> Target {
    let text = format!("{path}: ");
    text.parse::<Contents>()
        .unwrap()
        .targets
        .into_iter()
        .next()
        .unwrap()
}

fn env_pair(key: &str, value: Option<&str>) -> (EnvKey, Option<EnvVal>) {
    let escape = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
    };
    let mut line = format!("# env-dep:{}", escape(key).replace('=', "\\="));
    if let Some(value) = value {
        line.push('=');
        line.push_str(&escape(value));
    }
    line.parse::<Contents>()
        .unwrap()
        .env
        .into_iter()
        .next()
        .unwrap()
}

fn checksum(text: &str) -> Digest {
    let input = format!("a:\n# checksum:{text} file_len:3 a");
    input
        .parse::<Contents>()
        .unwrap()
        .sources
        .into_values()
        .next()
        .unwrap()
        .unwrap()
        .digest
}

#[test]
fn rejects_duplicate_output_rules() {
    let text = r#"
app: src/main.rs
app: src/main.rs
src/main.rs:
"#;
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(
        observed.map_err(|error| error.details),
        Err(ParseErrorKind::DuplicateTarget)
    );
}

#[test]
fn rejects_duplicate_environment_keys() {
    let text = r#"
# env-dep:MODE=debug
# env-dep:MODE=release
"#;
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(
        observed.map_err(|error| error.details),
        Err(ParseErrorKind::DuplicateEnvKey)
    );
}

#[test]
fn preserves_colon_in_input() -> anyhow::Result<()> {
    let text = r#"
app: 0:
0::
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([(source("0:"), None)]),
        targets: IndexSet::from([target("app")]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn preserves_nonbreaking_space_in_input() -> anyhow::Result<()> {
    let text = "app: \u{a0}①\n\u{a0}①:\n";
    let expected = Ok(Contents {
        sources: IndexMap::from([(source("\u{a0}①"), None)]),
        targets: IndexSet::from([target("app")]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn parses_escaped_spaces_windows_paths_and_crlf() -> anyhow::Result<()> {
    let text = r#"
C:\build\app.exe: C:\src\main.rs dir\ with\ spaces/lib.rs
C:\src\main.rs:
dir\ with\ spaces/lib.rs:
"#
    .replace('\n', "\r\n");
    let expected = Ok(Contents {
        sources: IndexMap::from([
            (source(r"C:\src\main.rs"), None),
            (source("dir with spaces/lib.rs"), None),
        ]),
        targets: IndexSet::from([target(r"C:\build\app.exe")]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn preserves_environment_values_and_unescapes_names_and_values() -> anyhow::Result<()> {
    let text = r#"
# env-dep:UNSET
# env-dep:EMPTY=
# env-dep:VALUE=one=two
# env-dep:ESC\nAPED=line\nreturn\rslash\\
# env-dep:UNICODE=こんにちは
"#;
    let expected = Ok(Contents {
        sources: IndexMap::new(),
        targets: IndexSet::new(),
        env: IndexMap::from([
            env_pair("UNSET", None),
            env_pair("EMPTY", Some("")),
            env_pair("VALUE", Some("one=two")),
            env_pair("ESC\nAPED", Some("line\nreturn\rslash\\")),
            env_pair("UNICODE", Some("こんにちは")),
        ]),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn reads_environment_comment_with_make_syntax_and_crlf() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs
src/main.rs:
# env-dep:VALUE=$HOME$(NAME)
"#
    .replace('\n', "\r\n");
    let expected = Ok(Contents {
        sources: IndexMap::from([(source("src/main.rs"), None)]),
        targets: IndexSet::from([target("app")]),
        env: IndexMap::from([env_pair("VALUE", Some("$HOME$(NAME)"))]),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn rejects_invalid_environment_escape_in_name() -> anyhow::Result<()> {
    let text = r"# env-dep:BAD\t=value";
    let expected = Err(ParseErrorKind::InvalidEnvEscape);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed.map_err(|error| error.details));
    Ok(())
}

#[test]
fn rejects_invalid_environment_escape_in_value() -> anyhow::Result<()> {
    let text = r"# env-dep:NAME=value\t";
    let expected = Err(ParseErrorKind::InvalidEnvEscape);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed.map_err(|error| error.details));
    Ok(())
}

#[test]
fn rejects_unterminated_environment_escape_in_value() -> anyhow::Result<()> {
    let text = r"# env-dep:NAME=value\";
    let expected = Err(ParseErrorKind::UnterminatedEnvEscape);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed.map_err(|error| error.details));
    Ok(())
}

#[test]
fn rejects_unterminated_environment_escape_in_name() -> anyhow::Result<()> {
    let text = r"# env-dep:NAME\";
    let expected = Err(ParseErrorKind::UnterminatedEnvEscape);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed.map_err(|error| error.details));
    Ok(())
}

#[test]
fn rejects_input_ending_in_backslash() -> anyhow::Result<()> {
    let text = r"app: src/main.rs\";
    let expected = Err(ParseErrorKind::SourceEndsWithBackslash);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed.map_err(|error| error.details));
    Ok(())
}

#[test]
fn exposes_checksum_parse_errors() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs
src/main.rs:
# checksum:sha256=invalid file_len:3 src/main.rs
"#;
    let expected = Err(ParseErrorKind::InvalidChksumDigest);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed.map_err(|error| error.details));
    Ok(())
}

#[test]
fn attaches_checksum_to_input_with_escaped_spaces() -> anyhow::Result<()> {
    let text = r#"
app: dir\ with\ spaces/lib.rs

dir\ with\ spaces/lib.rs:
# checksum:sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad file_len:3 dir\ with\ spaces/lib.rs
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([(
            source("dir with spaces/lib.rs"),
            Some(Metadata {
                file_len: 3,
                digest: checksum(SHA256),
            }),
        )]),
        targets: IndexSet::from([target("app")]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn accepts_empty_input() -> anyhow::Result<()> {
    let text = r#"
"#;
    let expected = Ok(Contents::default());
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn reads_input_rule_without_outputs() -> anyhow::Result<()> {
    let text = r#"
src/main.rs:
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([(source("src/main.rs"), None)]),
        targets: IndexSet::new(),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn accepts_empty_output_rule() -> anyhow::Result<()> {
    let text = r#"
app: "#;
    let expected = Ok(Contents {
        sources: IndexMap::new(),
        targets: IndexSet::from([target("app")]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn parses_cargo_book_absolute_path_example() -> anyhow::Result<()> {
    // https://doc.rust-lang.org/cargo/reference/build-cache.html#dep-info-files
    let text = r#"
/path/to/myproj/target/debug/foo: /path/to/myproj/src/lib.rs /path/to/myproj/src/main.rs
/path/to/myproj/src/lib.rs:
/path/to/myproj/src/main.rs:
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([
            (source("/path/to/myproj/src/lib.rs"), None),
            (source("/path/to/myproj/src/main.rs"), None),
        ]),
        targets: IndexSet::from([target("/path/to/myproj/target/debug/foo")]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn parses_rustc_writer_rule_layout() -> anyhow::Result<()> {
    // rustc writes one rule per output, then one input-only rule per input.
    // https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_interface/passes.rs.html
    let text = r#"
target/debug/deps/libdemo.rlib: src/lib.rs src/generated\ code.rs

target/debug/deps/libdemo.rmeta: src/lib.rs src/generated\ code.rs

src/lib.rs:
src/generated\ code.rs:
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([
            (source("src/lib.rs"), None),
            (source("src/generated code.rs"), None),
        ]),
        targets: IndexSet::from([
            target("target/debug/deps/libdemo.rlib"),
            target("target/debug/deps/libdemo.rmeta"),
        ]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn preserves_unescaped_dollar_and_hash_in_rustc_input_paths() -> anyhow::Result<()> {
    // rustc escapes spaces in input paths, leaving `$` and `#` unchanged.
    // https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_interface/passes.rs.html
    let text = r#"
app: src/$build#1.rs src/a\ b.rs

src/$build#1.rs:
src/a\ b.rs:
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([
            (source("src/$build#1.rs"), None),
            (source("src/a b.rs"), None),
        ]),
        targets: IndexSet::from([target("app")]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn rejects_out_of_order_and_unrecognized_lines() {
    for text in [
        "# ordinary comment",
        "# env-dep:A=1
app: ",
        "app: a
a:
# env-dep:A=1
a:",
        "app: a
a:
# checksum:sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad file_len:3 a
# env-dep:A=1",
    ] {
        assert!(text.parse::<Contents>().is_err(), "{text}");
    }
}

#[test]
fn rejects_inconsistent_rules_and_checksums() {
    let checksum = format!("# checksum:{SHA256} file_len:3 a");
    for text in [
        "app: a
other: b
a:"
        .to_owned(),
        "app: a
"
        .to_owned(),
        format!(
            "app: b
b:
{checksum}"
        ),
        format!(
            "app: a
a:
{checksum}
{checksum}"
        ),
    ] {
        assert!(text.parse::<Contents>().is_err(), "{text}");
    }
}

#[test]
fn rejects_incomplete_checksum_records() {
    for suffix in [
        "",
        " ",
        " length:3 a",
        " file_len:invalid a",
        " file_len:-1 a",
        " file_len:18446744073709551616 a",
        " file_len:3",
    ] {
        let text = format!(
            "app: a
a:
# checksum:{SHA256}{suffix}"
        );
        assert!(text.parse::<Contents>().is_err(), "{text}");
    }
}

#[test]
fn parses_complete_ordered_file() -> anyhow::Result<()> {
    let text = format!(
        "app: a b
app.rmeta: a b
a:
b:
# env-dep:MODE=debug
# checksum:{SHA256} file_len:3 a
"
    );
    let info: Contents = text.parse()?;
    assert_eq!(info.targets.len(), 2);
    assert_eq!(info.sources.len(), 2);
    assert_eq!(info.sources["a"].as_ref().unwrap().file_len, 3);
    assert!(info.sources["b"].is_none());
    assert_eq!(info.env["MODE"].as_ref().unwrap().as_str(), "debug");
    Ok(())
}

#[test]
fn exposes_structured_invalid_line_errors() {
    let text = "# checksum:sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad file_len:invalid src.rs";
    let expected = Err(ParseErrorKind::InvalidChksumFileLength);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed.map_err(|error| error.details));
}

#[test]
fn reports_reachable_path_and_checksum_errors() {
    for (text, expected) in [
        (": a", ParseErrorKind::EmptyTarget),
        (":", ParseErrorKind::EmptySource),
        (r"src\:", ParseErrorKind::SourceEndsWithBackslash),
        ("bad\rpath:", ParseErrorKind::SourceContainsCarriageReturn),
        (
            "bad\rtarget: a",
            ParseErrorKind::TargetContainsCarriageReturn,
        ),
        (
            "# checksum:unknown=00 file_len:1 a",
            ParseErrorKind::InvalidChksumAlgorithm,
        ),
        ("# checksum:missing", ParseErrorKind::InvalidChksumFormat),
    ] {
        let observed: Result<Contents, ParseError> = text.parse();
        assert_eq!(
            observed.map_err(|error| error.details),
            Err(expected),
            "{text:?}"
        );
    }
}

#[test]
fn checks_duplicates_and_section_order() {
    let checksum = format!("# checksum:{SHA256} file_len:3 a");
    for (text, expected) in [
        ("app: a a".to_owned(), ParseErrorKind::DuplicateSource),
        (
            "app: a
a:
a:"
            .to_owned(),
            ParseErrorKind::DuplicateSourceRule,
        ),
        (
            format!(
                "app: a
a:
{checksum}
{checksum}"
            ),
            ParseErrorKind::DuplicateChksum,
        ),
        (
            format!(
                "app: a
a:
# checksum:{SHA256} file_len:3 b"
            ),
            ParseErrorKind::UnknownChksumSource,
        ),
        (
            "app: a
a:
# env-dep:K=V
a:"
            .to_owned(),
            ParseErrorKind::MisplacedLine,
        ),
    ] {
        let observed: Result<Contents, ParseError> = text.parse();
        assert_eq!(
            observed.map_err(|error| error.details),
            Err(expected),
            "{text}"
        );
    }
}

#[test]
fn accepts_reordered_source_rules_and_whitespace_between_sections() -> anyhow::Result<()> {
    let text = r#"
app: a b

b:
a:

# env-dep:K=V

"#;
    let observed: Contents = text.parse()?;
    assert_eq!(
        observed
            .sources
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
        ["b", "a"]
    );
    Ok(())
}

#[test]
fn reports_one_based_character_positions() {
    let text = concat!("\r\n", "# env-dep:é=bad\\t\r\n");
    let error = text.parse::<Contents>().unwrap_err();
    assert_eq!(error.line, 2);
    assert_eq!(error.column, 16);
    assert_eq!(error.details, ParseErrorKind::InvalidEnvEscape);
    assert_eq!(
        error.to_string(),
        "line 2, column 16: unknown environment escape character"
    );
}

#[test]
fn reports_positions_for_line_and_missing_rule_errors() {
    for (text, line, column, details) in [
        ("app: a", 1, 6, ParseErrorKind::MissingSourceRule),
        ("app: a\n", 1, 6, ParseErrorKind::MissingSourceRule),
        (
            "app: a\na:\n# env-dep:X=Y\na:",
            4,
            1,
            ParseErrorKind::MisplacedLine,
        ),
        (
            "# checksum:sha256=invalid file_len:1 a",
            1,
            19,
            ParseErrorKind::InvalidChksumDigest,
        ),
    ] {
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!(
            (error.line, error.column, error.details),
            (line, column, details)
        );
    }
}

#[test]
fn locates_errors_in_original_unicode_and_escaped_text() {
    use ParseErrorKind::*;
    for (text, line, column, kind) in [
        ("é: a a", 1, 6, DuplicateSource),
        (r"é: a\ b a\ b", 1, 9, DuplicateSource),
        ("é: a  b", 1, 6, EmptySource),
        ("é: a ", 1, 6, EmptySource),
        ("é: a\na:\nb:", 3, 1, UnexpectedSourceRule),
        ("é: a b\na:", 1, 6, MissingSourceRule),
        ("é: a\nother: b", 2, 8, TargetSourcesMismatch),
        (r"é: a\ b\", 1, 8, SourceEndsWithBackslash),
        ("é: a\\ b\r:", 1, 8, SourceContainsCarriageReturn),
        ("a\\ b\r:", 1, 5, SourceContainsCarriageReturn),
        (r"# env-dep:é\=clé=ok\n\t", 1, 22, InvalidEnvEscape),
        (r"# env-dep:é\=clé=ok\n\", 1, 22, UnterminatedEnvEscape),
        ("# env-dep:é=1\n# env-dep:é=2", 2, 11, DuplicateEnvKey),
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
fn validates_local_duplicates_before_rule_consistency() {
    for (text, column, kind) in [
        ("app: a\nother: a a", 10, ParseErrorKind::DuplicateSource),
        ("app: a\napp: b", 1, ParseErrorKind::DuplicateTarget),
        ("app: a\napp: b b", 8, ParseErrorKind::DuplicateSource),
    ] {
        let error = text.parse::<Contents>().unwrap_err();
        assert_eq!((error.line, error.column, error.details), (2, column, kind));
    }
}

#[test]
fn locates_checksum_fields_and_source_paths() {
    use ParseErrorKind::*;
    let prefix = format!("# checksum:{SHA256}");
    let length_column = prefix.chars().count() + 2;
    let path_column = length_column + "file_len:3 ".len();
    for (text, line, column, kind) in [
        (
            prefix.clone(),
            1,
            prefix.chars().count() + 1,
            InvalidChksumFileLength,
        ),
        (
            format!("{prefix} file_len:no a"),
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
            format!("{prefix} file_len:3 é"),
            1,
            path_column,
            UnknownChksumSource,
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
