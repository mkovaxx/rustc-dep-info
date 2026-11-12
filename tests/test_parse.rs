use indexmap::{IndexMap, IndexSet};

use rustc_dep_info::{Checksum, ChecksumAlgo, Contents, InvalidChecksum, Metadata, ParseError};

const SHA256: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn deduplicates_output_rules() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs
app: src/main.rs
src/main.rs:
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([("src/main.rs".try_into()?, None)]),
        targets: IndexSet::from(["app".try_into()?]),
        env: IndexMap::new(),
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
        sources: IndexMap::new(),
        targets: IndexSet::new(),
        env: IndexMap::from([("MODE".into(), Some("release".into()))]),
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
        sources: IndexMap::from([("0:".try_into()?, None)]),
        targets: IndexSet::from(["app".try_into()?]),
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
        sources: IndexMap::from([("\u{a0}①".try_into()?, None)]),
        targets: IndexSet::from(["app".try_into()?]),
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
            (r"C:\src\main.rs".try_into()?, None),
            ("dir with spaces/lib.rs".try_into()?, None),
        ]),
        targets: IndexSet::from([r"C:\build\app.exe".try_into()?]),
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
src/main.rs:
# env-dep:VALUE=$HOME$(NAME)
"#
    .replace('\n', "\r\n");
    let expected = Ok(Contents {
        sources: IndexMap::from([("src/main.rs".try_into()?, None)]),
        targets: IndexSet::from(["app".try_into()?]),
        env: IndexMap::from([("VALUE".into(), Some("$HOME$(NAME)".into()))]),
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
fn exposes_checksum_parse_errors() -> anyhow::Result<()> {
    let text = r#"
app: src/main.rs
src/main.rs:
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
        sources: IndexMap::from([(
            "dir with spaces/lib.rs".try_into()?,
            Some(Metadata {
                file_len: 3,
                checksum: SHA256.parse::<Checksum>()?,
            }),
        )]),
        targets: IndexSet::from(["app".try_into()?]),
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
        sources: IndexMap::from([("src/main.rs".try_into()?, None)]),
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
        targets: IndexSet::from(["app".try_into()?]),
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
            ("/path/to/myproj/src/lib.rs".try_into()?, None),
            ("/path/to/myproj/src/main.rs".try_into()?, None),
        ]),
        targets: IndexSet::from(["/path/to/myproj/target/debug/foo".try_into()?]),
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
            ("src/lib.rs".try_into()?, None),
            ("src/generated code.rs".try_into()?, None),
        ]),
        targets: IndexSet::from([
            "target/debug/deps/libdemo.rlib".try_into()?,
            "target/debug/deps/libdemo.rmeta".try_into()?,
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
            ("src/$build#1.rs".try_into()?, None),
            ("src/a b.rs".try_into()?, None),
        ]),
        targets: IndexSet::from(["app".try_into()?]),
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
        "app: a b
b:
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
