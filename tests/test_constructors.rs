use rustc_dep_info::{EnvKey, EnvVal, Source, SourceError, Target, TargetError};

#[test]
fn source_constructor_enforces_invariants() {
    for (text, expected) in [
        ("", SourceError::Empty),
        (r"src/main.rs\", SourceError::EndsWithBackslash),
        ("src\nmain.rs", SourceError::ContainsLineBreak),
        ("src\rmain.rs", SourceError::ContainsLineBreak),
        ("src/main.rs\n", SourceError::ContainsLineBreak),
        ("src/main.rs\r\n", SourceError::ContainsLineBreak),
    ] {
        assert_eq!(Source::new(text), Err(expected), "{text:?}");
    }
}

#[test]
fn target_constructor_enforces_invariants() {
    for (text, expected) in [
        ("", TargetError::Empty),
        ("# env-dep", TargetError::ReservedCommentPrefix),
        ("# checksum", TargetError::ReservedCommentPrefix),
        ("# env-dep:", TargetError::ReservedCommentPrefix),
        ("# env-dep:FOO=bar", TargetError::ReservedCommentPrefix),
        ("# env-dep:FOO: ", TargetError::ReservedCommentPrefix),
        ("# checksum:", TargetError::ReservedCommentPrefix),
        (
            "# checksum:sha256=invalid",
            TargetError::ReservedCommentPrefix,
        ),
        ("app: output", TargetError::ContainsRuleSeparator),
        ("app: ", TargetError::ContainsRuleSeparator),
        ("build\napp", TargetError::ContainsLineBreak),
        ("build\rapp", TargetError::ContainsLineBreak),
        ("app\n", TargetError::ContainsLineBreak),
        ("app\r\n", TargetError::ContainsLineBreak),
    ] {
        assert_eq!(Target::new(text), Err(expected), "{text:?}");
    }
}

#[test]
fn path_constructors_preserve_decoded_text() {
    for text in [
        "src/main.rs",
        " dir with spaces/é.rs ",
        r"C:\src\main.rs",
        r"dir\ with\ spaces/file.rs",
        r"literal\n.rs",
        "0:",
        " ",
    ] {
        assert_eq!(Source::new(text).unwrap().as_str(), text);
        assert_eq!(Source::new(text.to_owned()).unwrap().into_string(), text);
        assert_eq!(Target::new(text).unwrap().as_str(), text);
        assert_eq!(Target::new(text.to_owned()).unwrap().into_string(), text);
    }
    // The two path types have different restrictions.
    assert_eq!(Source::new("a: b").unwrap().as_str(), "a: b");
    assert_eq!(Target::new(r"build\").unwrap().as_str(), r"build\");
}

#[test]
fn environment_constructors_preserve_arbitrary_decoded_text() {
    for text in ["", "MODE", "a=b", " \t ", "é\n\r\0", r"literal\n\=\\"] {
        assert_eq!(EnvKey::new(text).as_str(), text);
        assert_eq!(EnvKey::new(text.to_owned()).into_string(), text);
        assert_eq!(EnvVal::new(text).as_str(), text);
        assert_eq!(EnvVal::new(text.to_owned()).into_string(), text);
    }
}
