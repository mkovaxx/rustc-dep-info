use std::{fmt, str};

use crate::{Contents, Digest, Metadata};

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
        if let Some(Metadata {
            file_len,
            digest: checksum,
        }) = metadata
        {
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
    // rustc writes each space in a source path as `\ `, in both rules and checksum comments.
    path.replace(' ', "\\ ")
}

fn print_env(text: &str) -> String {
    // rustc tries to fit env var names and values all on a single line, which means it needs to
    // escape `\r` and `\n`. The escape syntax used is "\n", so `\` also needs to be escaped.
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (algorithm, value) = match self {
            Digest::Sha256(value) => ("sha256", value),
            Digest::Blake3(value) => ("blake3", value),
        };
        let mut encoded = [0; 64];
        hex::encode_to_slice(value, &mut encoded).map_err(|_| fmt::Error)?;
        write!(
            f,
            "{algorithm}={}",
            str::from_utf8(&encoded).map_err(|_| fmt::Error)?
        )
    }
}
