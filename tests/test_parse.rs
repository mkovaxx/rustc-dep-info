use indexmap::{IndexMap, IndexSet};

use rustc_dep_info::{
    Contents, Digest, EnvKey, EnvVal, Metadata, ParseError, ParseErrorKind, Source, Target,
};

const SHA256: &str = "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn preserves_colon_in_input() -> anyhow::Result<()> {
    let text = r#"
app: 0:
0::
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([(Source::new("0:")?, None)]),
        targets: IndexSet::from([Target::new("app")?]),
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
        sources: IndexMap::from([(Source::new("\u{a0}①")?, None)]),
        targets: IndexSet::from([Target::new("app")?]),
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
            (Source::new(r"C:\src\main.rs")?, None),
            (Source::new("dir with spaces/lib.rs")?, None),
        ]),
        targets: IndexSet::from([Target::new(r"C:\build\app.exe")?]),
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
            (EnvKey::new("UNSET"), None),
            (EnvKey::new("EMPTY"), Some(EnvVal::new(""))),
            (EnvKey::new("VALUE"), Some(EnvVal::new("one=two"))),
            (
                EnvKey::new("ESC\nAPED"),
                Some(EnvVal::new("line\nreturn\rslash\\")),
            ),
            (EnvKey::new("UNICODE"), Some(EnvVal::new("こんにちは"))),
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
        sources: IndexMap::from([(Source::new("src/main.rs")?, None)]),
        targets: IndexSet::from([Target::new("app")?]),
        env: IndexMap::from([(EnvKey::new("VALUE"), Some(EnvVal::new("$HOME$(NAME)")))]),
    });
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
            Source::new("dir with spaces/lib.rs")?,
            Some(Metadata {
                file_len: 3,
                digest: Digest::Sha256([
                    0xBA, 0x78, 0x16, 0xBF, 0x8F, 0x01, 0xCF, 0xEA, 0x41, 0x41, 0x40, 0xDE, 0x5D,
                    0xAE, 0x22, 0x23, 0xB0, 0x03, 0x61, 0xA3, 0x96, 0x17, 0x7A, 0x9C, 0xB4, 0x10,
                    0xFF, 0x61, 0xF2, 0x00, 0x15, 0xAD,
                ]),
            }),
        )]),
        targets: IndexSet::from([Target::new("app")?]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn accepts_empty_input() -> anyhow::Result<()> {
    for text in ["", "\n", "\r\n", " \t\n\r\n"] {
        assert_eq!(text.parse::<Contents>()?, Contents::default(), "{text:?}");
    }
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
            (Source::new("/path/to/myproj/src/lib.rs")?, None),
            (Source::new("/path/to/myproj/src/main.rs")?, None),
        ]),
        targets: IndexSet::from([Target::new("/path/to/myproj/target/debug/foo")?]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn parses_rustc_writer_rule_layout() -> anyhow::Result<()> {
    // rustc writes one rule per target, then one source-only rule per source.
    // https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_interface/passes.rs.html
    let text = r#"
target/debug/deps/libdemo.rlib: src/lib.rs src/generated\ code.rs

target/debug/deps/libdemo.rmeta: src/lib.rs src/generated\ code.rs

src/lib.rs:
src/generated\ code.rs:
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([
            (Source::new("src/lib.rs")?, None),
            (Source::new("src/generated code.rs")?, None),
        ]),
        targets: IndexSet::from([
            Target::new("target/debug/deps/libdemo.rlib")?,
            Target::new("target/debug/deps/libdemo.rmeta")?,
        ]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
}

#[test]
fn preserves_unescaped_dollar_and_hash_in_rustc_input_paths() -> anyhow::Result<()> {
    // rustc escapes spaces in source paths, leaving `$` and `#` unchanged.
    // https://doc.rust-lang.org/nightly/nightly-rustc/src/rustc_interface/passes.rs.html
    let text = r#"
app: src/$build#1.rs src/a\ b.rs

src/$build#1.rs:
src/a\ b.rs:
"#;
    let expected = Ok(Contents {
        sources: IndexMap::from([
            (Source::new("src/$build#1.rs")?, None),
            (Source::new("src/a b.rs")?, None),
        ]),
        targets: IndexSet::from([Target::new("app")?]),
        env: IndexMap::new(),
    });
    let observed: Result<Contents, ParseError> = text.parse();
    assert_eq!(expected, observed);
    Ok(())
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
fn accepts_layouts_with_optional_sections() -> anyhow::Result<()> {
    for targets in [false, true] {
        for sources in [false, true] {
            for env in [false, true] {
                for checksums in [false, true] {
                    // A checksum needs a source rule, but neither requires a target rule.
                    if checksums && !sources {
                        continue;
                    }
                    let mut text = String::new();
                    let mut expected = Contents::default();
                    if targets {
                        text.push_str(if sources { "app: a\n" } else { "app: \n" });
                        expected.targets.insert(Target::new("app")?);
                    }
                    if sources {
                        text.push_str("a:\n");
                        expected.sources.insert(Source::new("a")?, None);
                    }
                    if env {
                        text.push_str("# env-dep:K=V\n");
                        expected
                            .env
                            .insert(EnvKey::new("K"), Some(EnvVal::new("V")));
                    }
                    if checksums {
                        text.push_str(concat!(
                            "# checksum:sha256=",
                            "0000000000000000000000000000000000000000000000000000000000000000",
                            " file_len:0 a\n",
                        ));
                        expected.sources.insert(
                            Source::new("a")?,
                            Some(Metadata {
                                file_len: 0,
                                digest: Digest::Sha256([0; 32]),
                            }),
                        );
                    }
                    for text in [text.as_str(), text.strip_suffix('\n').unwrap_or(&text)] {
                        assert_eq!(text.parse::<Contents>()?, expected, "{text:?}");
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn accepts_checksum_file_length_boundaries() -> anyhow::Result<()> {
    for length in [0, u64::MAX] {
        let text = format!("a:\n# checksum:{SHA256} file_len:{length} a\n");
        let contents: Contents = text.parse()?;
        assert_eq!(contents.sources["a"].as_ref().unwrap().file_len, length);
    }
    Ok(())
}

#[test]
fn parses_checksum_algorithms_and_hex_case() -> anyhow::Result<()> {
    let abc = [
        0xBA, 0x78, 0x16, 0xBF, 0x8F, 0x01, 0xCF, 0xEA, 0x41, 0x41, 0x40, 0xDE, 0x5D, 0xAE, 0x22,
        0x23, 0xB0, 0x03, 0x61, 0xA3, 0x96, 0x17, 0x7A, 0x9C, 0xB4, 0x10, 0xFF, 0x61, 0xF2, 0x00,
        0x15, 0xAD,
    ];
    for (checksum, expected) in [
        (
            "sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            Digest::Sha256(abc),
        ),
        (
            "sha256=BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
            Digest::Sha256(abc),
        ),
        (
            "blake3=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            Digest::Blake3(abc),
        ),
        (
            "blake3=BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
            Digest::Blake3(abc),
        ),
    ] {
        let text = format!("a:\n# checksum:{checksum} file_len:3 a");
        let contents: Contents = text.parse()?;
        assert_eq!(
            contents.sources["a"].as_ref().unwrap().digest,
            expected,
            "{checksum}"
        );
    }
    Ok(())
}

#[test]
fn round_trips_targets_near_reserved_comment_prefixes() -> anyhow::Result<()> {
    for path in [
        "# env-dependency",
        "# env-dep-file",
        "# checksum-file",
        "#env-dep:FOO",
        "#checksum:sha256=invalid",
        "./# env-dep:FOO",
        "./# checksum:sha256=invalid",
    ] {
        let contents = Contents {
            targets: IndexSet::from([Target::new(path)?]),
            ..Contents::default()
        };
        let text = contents.to_string();
        assert_eq!(text.parse::<Contents>()?, contents, "{path:?}");
    }
    Ok(())
}

#[test]
fn round_trips_sources_with_reserved_comment_prefixes() -> anyhow::Result<()> {
    let contents = Contents {
        sources: IndexMap::from([
            (Source::new("# env-dep")?, None),
            (Source::new("# checksum")?, None),
            (Source::new("# env-dep:FOO=bar")?, None),
            (Source::new("# checksum:sha256=invalid")?, None),
        ]),
        ..Contents::default()
    };
    let text = contents.to_string();
    assert_eq!(text.parse::<Contents>()?, contents);
    Ok(())
}
