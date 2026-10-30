use std::{fs, path::Path};

use crate::{Contents, ParseError};

impl Contents {
    /// Read and parse a dep-info file.
    pub fn read_file(path: &Path) -> Result<Contents, FileReadError> {
        Ok(fs::read_to_string(path)?.parse()?)
    }

    /// Print these contents to a dep-info file.
    pub fn write_file(&self, path: &Path) -> Result<(), FileWriteError> {
        Ok(fs::write(path, self.to_string())?)
    }
}

/// An error reading or parsing a dep-info file.
#[derive(Debug, thiserror::Error)]
pub enum FileReadError {
    /// Reading the file failed.
    #[error("could not read dep-info file: {0}")]
    Io(#[from] std::io::Error),
    /// Parsing the file contents failed.
    #[error("could not parse dep-info file: {0}")]
    Parse(#[from] ParseError),
}

/// An error writing a dep-info file.
#[derive(Debug, thiserror::Error)]
pub enum FileWriteError {
    /// Writing the file failed.
    #[error("could not write dep-info file: {0}")]
    Io(#[from] std::io::Error),
}
