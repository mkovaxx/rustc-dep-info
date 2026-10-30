use std::{fmt, str};

use serde::Serialize;

use crate::{Checksum, ChecksumAlgo, Contents, Metadata};

impl fmt::Display for Contents {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print_contents(self, f)
    }
}

fn print_contents(contents: &Contents, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for output in &contents.targets {
        write!(f, "{}: ", output.as_str())?;
        for (index, path) in contents.sources.keys().enumerate() {
            if index > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", print_input_path(path.as_str()))?;
        }
        writeln!(f)?;
    }

    for path in contents.sources.keys() {
        writeln!(f, "{}:", print_input_path(path.as_str()))?;
    }

    for (name, value) in &contents.env {
        write!(
            f,
            "# env-dep:{}",
            print_env(name.as_str()).replace('=', "\\=")
        )?;
        if let Some(value) = value {
            write!(f, "={}", print_env(value.as_str()))?;
        }
        writeln!(f)?;
    }

    for (path, metadata) in &contents.sources {
        if let Some(Metadata { file_len, checksum }) = metadata {
            writeln!(
                f,
                "# checksum:{checksum} file_len:{file_len} {}",
                print_input_path(path.as_str())
            )?;
        }
    }

    Ok(())
}

fn print_input_path(path: &str) -> String {
    // rustc writes each space in an input path as `\ `, in both rules and checksum comments.
    path.replace(' ', "\\ ")
}

fn print_env(text: &str) -> String {
    // rustc tries to fit env var names and values all on a single line, which means it needs to
    // escape `\r` and `\n`. The escape syntax used is "\n", so `\` also needs to be escaped.
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut checksum = [0; 64];
        let hash_len = self.algo.hash_len();
        hex::encode_to_slice(&self.value[0..hash_len], &mut checksum[0..(hash_len * 2)])
            .map_err(|_| fmt::Error)?;
        write!(
            f,
            "{}={}",
            self.algo,
            str::from_utf8(&checksum[0..(hash_len * 2)]).unwrap_or_default()
        )
    }
}

impl fmt::Display for ChecksumAlgo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ChecksumAlgo::Sha256 => "sha256",
            ChecksumAlgo::Blake3 => "blake3",
        })
    }
}

impl Serialize for Checksum {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
