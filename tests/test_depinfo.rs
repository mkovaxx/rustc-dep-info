#![cfg(feature = "arbitrary")]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::path::PathBuf;

use depinfo::RustcDepInfo;
use proptest::prelude::*;
use rustc_dep_info::Contents;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1024,
        failure_persistence: None,
        .. ProptestConfig::default()
    })]

    #[test]
    fn depinfo_transparency(case in rustc_case()) {
        let text = render_rustc_dep_info(&case);
        let Ok(before) = text.parse::<RustcDepInfo>() else {
            return Ok(());
        };

        let contents = text.parse::<Contents>();
        prop_assert!(contents.is_ok(), "our parser rejected rustc-shaped text: {contents:?}\n{text}");
        let printed = contents.unwrap().to_string();
        let after = printed.parse::<RustcDepInfo>();
        prop_assert!(after.is_ok(), "depinfo rejected printed text: {after:?}\noriginal:\n{text}\nprinted:\n{printed}");
        prop_assert_eq!(
            observed_data(before),
            observed_data(after.unwrap()),
            "original:\n{}\nprinted:\n{}", text, printed
        );
    }
}

#[derive(Debug)]
struct RustcCase {
    targets: BTreeSet<String>,
    sources: BTreeMap<String, Option<(u16, [u8; 32])>>,
    env: BTreeMap<String, Option<String>>,
    checksum_blake3: Option<bool>,
}

fn rustc_case() -> impl Strategy<Value = RustcCase> {
    (
        proptest::collection::btree_set(target_path(), 1..4),
        proptest::collection::btree_map(
            source_path(),
            proptest::option::of((any::<u16>(), any::<[u8; 32]>())),
            1..5,
        ),
        proptest::collection::btree_map(env_key(), proptest::option::of(env_value()), 0..4),
        proptest::option::of(any::<bool>()),
    )
        .prop_map(|(targets, sources, env, checksum_blake3)| RustcCase {
            targets,
            sources,
            env,
            checksum_blake3,
        })
}

fn target_path() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::string::string_regex("[a-z0-9_./ -]{1,12}").unwrap(),
        Just("build/app output".to_owned()),
        Just("# target".to_owned()),
    ]
}

fn source_path() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::string::string_regex("[a-z0-9_./ -]{1,12}").unwrap(),
        Just("src/with space.rs".to_owned()),
        Just("src/back\\slash.rs".to_owned()),
        Just("src/non\u{a0}breaking.rs".to_owned()),
        Just("src/#hash.rs".to_owned()),
        Just("src/$dollar.rs".to_owned()),
    ]
}

fn env_key() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Z_][A-Z0-9_]{0,7}").unwrap()
}

fn env_value() -> impl Strategy<Value = String> {
    prop_oneof![
        proptest::string::string_regex("[a-z0-9 =:#$]{0,12}").unwrap(),
        Just("one\\two".to_owned()),
        Just("first\nsecond\rthird".to_owned()),
    ]
}

// Mirrors write_deps_to_file and its escaping helpers in rustc revision 16d1ffac17ad8b739e32483ac2e65fe7e1804977:
// https://github.com/rust-lang/rust/blob/16d1ffac17ad8b739e32483ac2e65fe7e1804977/compiler/rustc_interface/src/passes.rs
fn render_rustc_dep_info(case: &RustcCase) -> String {
    let mut text = String::new();
    let sources: Vec<_> = case
        .sources
        .keys()
        .map(|path| path.replace(' ', "\\ "))
        .collect();

    for target in &case.targets {
        writeln!(text, "{target}: {}\n", sources.join(" ")).unwrap();
    }
    for source in &sources {
        writeln!(text, "{source}:").unwrap();
    }

    if !case.env.is_empty() {
        let mut env: Vec<_> = case
            .env
            .iter()
            .map(|(key, value)| (escape_env(key), value.as_ref().map(|v| escape_env(v))))
            .collect();
        env.sort_unstable();
        writeln!(text).unwrap();
        for (key, value) in env {
            write!(text, "# env-dep:{key}").unwrap();
            if let Some(value) = value {
                write!(text, "={value}").unwrap();
            }
            writeln!(text).unwrap();
        }
    }

    if let Some(blake3) = case.checksum_blake3 {
        let algorithm = if blake3 { "blake3" } else { "sha256" };
        for (source, metadata) in &case.sources {
            if let Some((file_len, digest)) = metadata {
                writeln!(
                    text,
                    "# checksum:{algorithm}={} file_len:{file_len} {}",
                    hex::encode(digest),
                    source.replace(' ', "\\ ")
                )
                .unwrap();
            }
        }
    }

    text
}

fn escape_env(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn observed_data(info: RustcDepInfo) -> (BTreeSet<PathBuf>, BTreeMap<String, Option<String>>) {
    (
        info.files.into_iter().collect(),
        info.env.into_iter().collect(),
    )
}
