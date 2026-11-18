use proptest::prelude::*;
use rustc_dep_info::Contents;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        ..ProptestConfig::default()
    })]

    #[test]
    fn arbitrary_text_never_panics_and_error_locations_are_valid(text in input_text()) {
        // Proptest catches and shrinks panics, so parsing itself is part of the property.
        if let Err(error) = text.parse::<Contents>() {
            prop_assert!(error.line >= 1, "{error:?}");
            let line = text.split_inclusive('\n').nth(error.line - 1);
            prop_assert!(line.is_some(), "{error:?} in {text:?}");
            let line = line.unwrap();
            let line = line.strip_suffix("\r\n")
                .or_else(|| line.strip_suffix('\n'))
                .unwrap_or(line);
            // A missing token can point to the insertion position after the last character.
            prop_assert!((1..=line.chars().count() + 1).contains(&error.column),
                "{error:?} in {text:?}");
        }
    }
}

// Uniform random text usually fails at the first line. Mix it with record-shaped
// text and mutations of complete files to exercise field parsers and finalization.
fn input_text() -> impl Strategy<Value = String> {
    let arbitrary_text = proptest::collection::vec(any::<char>(), 0..512)
        .prop_map(|chars| chars.into_iter().collect::<String>());
    let record = (
        proptest::sample::select(vec!["", "app: ", "# env-dep:", "# checksum:"]),
        proptest::collection::vec(any::<char>(), 0..80),
        proptest::sample::select(vec!["", ":", "\\", "\r"]),
    )
        .prop_map(|(prefix, chars, suffix)| {
            format!("{prefix}{}{suffix}", chars.into_iter().collect::<String>())
        });
    let records = (
        proptest::collection::vec(record, 0..12),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(records, crlf, trailing_newline)| {
            let newline = if crlf { "\r\n" } else { "\n" };
            let mut text = records.join(newline);
            if trailing_newline {
                text.push_str(newline);
            }
            text
        });
    let mutated_file = (
        proptest::sample::select(vec![
            "app: a b\nother: a b\na:\nb:\n# env-dep:A=one\\ntwo\n",
            "a:\n# checksum:sha256=abababababababababababababababababababababababababababababababab file_len:3 a\n",
            "é: a\\ b\na\\ b:\n# env-dep:A\\=B=\\\\\r\n",
        ]),
        any::<usize>(),
        prop_oneof![any::<char>(), proptest::sample::select(vec!['\\', ' ', '\r', '\n', ':', '='])],
        0..3u8,
    ).prop_map(|(text, index, character, operation)| {
        let mut chars: Vec<_> = text.chars().collect();
        match operation {
            0 => {
                let index = index % (chars.len() + 1);
                chars.insert(index, character);
            }
            1 => {
                let index = index % chars.len();
                chars.remove(index);
            }
            _ => {
                let index = index % chars.len();
                chars[index] = character;
            }
        }
        chars.into_iter().collect::<String>()
    });
    prop_oneof![arbitrary_text, records, mutated_file]
}
