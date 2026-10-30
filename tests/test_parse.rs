use std::collections::{HashMap, HashSet};

use rustc_dep_info::{Checksum, ChecksumAlgo, Contents, InvalidChecksum, Metadata, ParseError};

const SHA256: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const BLAKE3: &str = "blake3=6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85";

#[test]
fn reads_all_rules_and_deduplicates_inputs() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs src/lib.rs src/main.rs

app.rmeta: ignored.rs

src/main.rs:
src/lib.rs:

# env-dep:AFTER_RULES=yes
"#;
    let expected = Ok(Contents {
        sources: HashMap::from([
            ("src/main.rs".try_into()?, None),
            ("src/lib.rs".try_into()?, None),
            ("ignored.rs".try_into()?, None),
        ]),
        targets: HashSet::from(["app".try_into()?, "app.rmeta".try_into()?]),
        env: HashMap::from([("AFTER_RULES".into(), Some("yes".into()))]),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn deduplicates_output_rules() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs
app: src/main.rs
"#;
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn later_environment_value_replaces_earlier_value() -> anyhow::Result<()> {
    let text = r#"
# env-dep:MODE=debug
# env-dep:MODE=release
"#;
    let expected = Ok(Contents {
        sources: HashMap::new(),
        targets: HashSet::new(),
        env: HashMap::from([("MODE".into(), Some("release".into()))]),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn reads_inputs_from_rule_with_space_in_output_path() -> anyhow::Result<()> {
    let text = r#"
# note: ignored.rs
build dir/app: src/main.rs dir\ with\ spaces/lib.rs
"#;
    let expected = Ok(Contents {
        sources: HashMap::from([
            ("ignored.rs".try_into()?, None),
            ("src/main.rs".try_into()?, None),
            ("dir with spaces/lib.rs".try_into()?, None),
        ]),
        targets: HashSet::from(["# note".try_into()?, "build dir/app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn preserves_colon_in_input() -> anyhow::Result<()> {
    let text = r#"
app: 0:
0::
"#;
    let expected = Ok(Contents {
        sources: HashMap::from([("0:".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn preserves_nonbreaking_space_in_input() -> anyhow::Result<()> {
    let text = "app: \u{a0}①\n\u{a0}①:\n";
    let expected = Ok(Contents {
        sources: HashMap::from([("\u{a0}①".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn parses_escaped_spaces_windows_paths_and_crlf() -> anyhow::Result<()> {
    let text = r#"
C:\build\app.exe: C:\src\main.rs dir\ with\ spaces/lib.rs
"#
    .replace('\n', "\r\n");
    let expected = Ok(Contents {
        sources: HashMap::from([
            (r"C:\src\main.rs".try_into()?, None),
            ("dir with spaces/lib.rs".try_into()?, None),
        ]),
        targets: HashSet::from([r"C:\build\app.exe".try_into()?]),
        env: HashMap::new(),
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
        sources: HashMap::new(),
        targets: HashSet::new(),
        env: HashMap::from([
            ("UNSET".into(), None),
            ("EMPTY".into(), Some("".into())),
            ("VALUE".into(), Some("one=two".into())),
            ("ESC\nAPED".into(), Some("line\nreturn\rslash\\".into())),
            ("UNICODE".into(), Some("こんにちは".into())),
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
# env-dep:VALUE=$HOME$(NAME)
"#
    .replace('\n', "\r\n");
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::from([("VALUE".into(), Some("$HOME$(NAME)".into()))]),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn rejects_invalid_environment_escape_in_name() -> anyhow::Result<()> {
    let text = r"# env-dep:BAD\t=value";
    let expected = Err(ParseError::InvalidEnvironmentEscape('t'));
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn rejects_invalid_environment_escape_in_value() -> anyhow::Result<()> {
    let text = r"# env-dep:NAME=value\t";
    let expected = Err(ParseError::InvalidEnvironmentEscape('t'));
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn rejects_unterminated_environment_escape_in_value() -> anyhow::Result<()> {
    let text = r"# env-dep:NAME=value\";
    let expected = Err(ParseError::UnterminatedEnvironmentEscape);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn rejects_unterminated_environment_escape_in_name() -> anyhow::Result<()> {
    let text = r"# env-dep:NAME\";
    let expected = Err(ParseError::UnterminatedEnvironmentEscape);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn rejects_input_ending_in_backslash() -> anyhow::Result<()> {
    let text = r"app: src/main.rs\";
    let expected = Err(ParseError::TrailingDependencyEscape);
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn parses_checksums_and_keeps_other_dependencies_without_metadata() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs src/lib.rs
# checksum:sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad file_len:3 src/main.rs
# checksum:blake3=6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85 file_len:42 extra.rs
"#;
    let expected = Ok(Contents {
        sources: HashMap::from([
            (
                "src/main.rs".try_into()?,
                Some(Metadata {
                    file_len: 3,
                    checksum: SHA256.parse::<Checksum>()?,
                }),
            ),
            ("src/lib.rs".try_into()?, None),
            (
                "extra.rs".try_into()?,
                Some(Metadata {
                    file_len: 42,
                    checksum: BLAKE3.parse::<Checksum>()?,
                }),
            ),
        ]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn checksum_records_survive_rule_insertion_and_later_records_replace_them() -> anyhow::Result<()> {
    let text = r#"
# checksum:sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad file_len:3 src/main.rs
app: src/main.rs
# checksum:blake3=6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85 file_len:4 src/main.rs
"#;
    let expected = Ok(Contents {
        sources: HashMap::from([(
            "src/main.rs".try_into()?,
            Some(Metadata {
                file_len: 4,
                checksum: BLAKE3.parse::<Checksum>()?,
            }),
        )]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn ignores_checksum_without_metadata() -> anyhow::Result<()> {
    let text = format!("app: src/main.rs\n# checksum:{SHA256} ");
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn ignores_checksum_with_incorrect_length_field() -> anyhow::Result<()> {
    let text = format!("app: src/main.rs\n# checksum:{SHA256} length:3 src/main.rs");
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn ignores_checksum_with_invalid_length() -> anyhow::Result<()> {
    let text = format!("app: src/main.rs\n# checksum:{SHA256} file_len:invalid src/main.rs");
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn ignores_checksum_with_negative_length() -> anyhow::Result<()> {
    let text = format!("app: src/main.rs\n# checksum:{SHA256} file_len:-1 src/main.rs");
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn ignores_checksum_with_overflowing_length() -> anyhow::Result<()> {
    let text =
        format!("app: src/main.rs\n# checksum:{SHA256} file_len:18446744073709551616 src/main.rs");
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn ignores_checksum_without_path() -> anyhow::Result<()> {
    let text = format!("app: src/main.rs\n# checksum:{SHA256} file_len:3");
    let expected = Ok(Contents {
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn exposes_checksum_parse_errors() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs
# checksum:sha256=invalid file_len:3 src/main.rs
"#;
    let expected = Err(ParseError::InvalidChecksum(
        InvalidChecksum::InvalidChecksum(ChecksumAlgo::Sha256),
    ));
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
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
        sources: HashMap::from([(
            "dir with spaces/lib.rs".try_into()?,
            Some(Metadata {
                file_len: 3,
                checksum: SHA256.parse::<Checksum>()?,
            }),
        )]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
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
fn accepts_ordinary_comment() -> anyhow::Result<()> {
    let text = r#"
# an ordinary comment
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
        sources: HashMap::from([("src/main.rs".try_into()?, None)]),
        targets: HashSet::new(),
        env: HashMap::new(),
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
        sources: HashMap::new(),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
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
"#;
    let expected = Ok(Contents {
        sources: HashMap::from([
            ("/path/to/myproj/src/lib.rs".try_into()?, None),
            ("/path/to/myproj/src/main.rs".try_into()?, None),
        ]),
        targets: HashSet::from(["/path/to/myproj/target/debug/foo".try_into()?]),
        env: HashMap::new(),
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
        sources: HashMap::from([
            ("src/lib.rs".try_into()?, None),
            ("src/generated code.rs".try_into()?, None),
        ]),
        targets: HashSet::from([
            "target/debug/deps/libdemo.rlib".try_into()?,
            "target/debug/deps/libdemo.rmeta".try_into()?,
        ]),
        env: HashMap::new(),
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
        sources: HashMap::from([
            ("src/$build#1.rs".try_into()?, None),
            ("src/a b.rs".try_into()?, None),
        ]),
        targets: HashSet::from(["app".try_into()?]),
        env: HashMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}
