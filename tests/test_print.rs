use indexmap::{IndexMap, IndexSet};
use rustc_dep_info::{Contents, Digest, EnvKey, EnvVal, Metadata, Source, Target};

#[test]
fn prints_complete_file_exactly() -> anyhow::Result<()> {
    let contents = Contents {
        targets: IndexSet::from([
            Target::new("build/app output")?,
            Target::new(r"C:\build\app.rmeta")?,
        ]),
        sources: IndexMap::from([
            (
                Source::new("src/é file.rs")?,
                Some(Metadata {
                    file_len: 0,
                    digest: Digest::Sha256([0xAB; 32]),
                }),
            ),
            (Source::new(r"src\lib.rs")?, None),
            (
                Source::new("src/main.rs")?,
                Some(Metadata {
                    file_len: u64::MAX,
                    digest: Digest::Blake3([0xCD; 32]),
                }),
            ),
        ]),
        env: IndexMap::from([
            (EnvKey::new("UNSET"), None),
            (EnvKey::new("EMPTY"), Some(EnvVal::new(""))),
            (
                EnvKey::new("A=B\nC\rD\\E"),
                Some(EnvVal::new("é=one\ntwo\rthree\\four")),
            ),
        ]),
    };
    let expected = concat!(
        "build/app output: src/é\\ file.rs src\\lib.rs src/main.rs\n",
        "C:\\build\\app.rmeta: src/é\\ file.rs src\\lib.rs src/main.rs\n",
        "src/é\\ file.rs:\n",
        "src\\lib.rs:\n",
        "src/main.rs:\n",
        "# env-dep:UNSET\n",
        "# env-dep:EMPTY=\n",
        "# env-dep:A\\=B\\nC\\rD\\\\E=é=one\\ntwo\\rthree\\\\four\n",
        "# checksum:sha256=abababababababababababababababababababababababababababababababab file_len:0 src/é\\ file.rs\n",
        "# checksum:blake3=cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd file_len:18446744073709551615 src/main.rs\n",
    );
    assert_eq!(contents.to_string(), expected);
    Ok(())
}

#[test]
fn formats_digests_with_algorithm_and_lowercase_hex() {
    for (digest, expected) in [
        (
            Digest::Sha256([0x12; 32]),
            "sha256=1212121212121212121212121212121212121212121212121212121212121212",
        ),
        (
            Digest::Blake3([0x34; 32]),
            "blake3=3434343434343434343434343434343434343434343434343434343434343434",
        ),
        (
            Digest::Sha256([0xAB; 32]),
            "sha256=abababababababababababababababababababababababababababababababab",
        ),
        (
            Digest::Blake3([0xCD; 32]),
            "blake3=cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
        ),
    ] {
        assert_eq!(digest.to_string(), expected);
    }
}
