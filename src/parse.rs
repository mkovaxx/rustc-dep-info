use std::str::FromStr;

use crate::{
    Checksum, ChecksumAlgo, Contents, EnvKey, EnvVal, InvalidChecksum, InvalidSource,
    InvalidTarget, Metadata, Source, Target,
};

/// An error parsing the contents of a rustc dep-info file.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// Target rules disagree on their source paths.
    #[error("target rules have different sources")]
    TargetSourcesMismatch,

    /// Source rules do not match the target rules.
    #[error("source rules do not match target rules")]
    SourceRulesMismatch,

    /// A checksum references an unknown source or repeats a source.
    #[error("invalid or duplicate checksum source: {0}")]
    ChecksumSource(String),

    /// A line appears misplaced.
    #[error("misplaced line: {0}")]
    MisplacedLine(String),

    /// A target rule has an empty target path.
    #[error("target path is empty")]
    EmptyTarget,

    /// A target path contains a line break.
    #[error("target path contains a line break")]
    TargetContainsLineBreak,

    /// A source path is empty.
    #[error("source path is empty")]
    EmptySource,

    /// A source path ends with a backslash.
    #[error("source path ends with a backslash")]
    SourceTrailingBackslash,

    /// A source path contains a line break.
    #[error("source path contains a line break")]
    SourceContainsLineBreak,

    /// A dependency path ends with an escape character.
    #[error("unterminated escaped dependency path")]
    TrailingDependencyEscape,

    /// A target rule has invalid syntax.
    #[error("invalid target rule syntax")]
    InvalidTargetRuleSyntax,

    /// A source rule has invalid syntax.
    #[error("invalid source rule syntax")]
    InvalidSourceRuleSyntax,

    /// An environment dependency contains an unknown escape.
    #[error("unknown environment escape character `{0}`")]
    InvalidEnvironmentEscape(char),

    /// An environment dependency ends with an escape character.
    #[error("unterminated environment escape character")]
    UnterminatedEnvironmentEscape,

    /// An environment dependency comment has invalid syntax.
    #[error("invalid env-dep comment syntax")]
    InvalidEnvDepSyntax,

    /// A checksum uses an unknown algorithm.
    #[error("unsupported checksum algorithm")]
    InvalidChecksumAlgorithm,

    /// A checksum digest is invalid for its algorithm.
    #[error("invalid checksum digest for {0}")]
    InvalidChecksumDigest(ChecksumAlgo),

    /// A checksum has an invalid format.
    #[error("invalid checksum format")]
    InvalidChecksumFormat,

    /// A checksum comment has an invalid or missing file length.
    #[error("invalid or missing checksum file length")]
    InvalidChecksumFileLength,

    /// A checksum comment has invalid syntax.
    #[error("invalid checksum comment syntax")]
    InvalidChecksumSyntax,

    /// A line has unrecognized syntax.
    #[error("unknown syntax: {0}")]
    UnknownLine(String),
}

fn invalid_source(error: InvalidSource) -> ParseError {
    match error {
        InvalidSource::Empty => ParseError::EmptySource,
        InvalidSource::TrailingBackslash => ParseError::SourceTrailingBackslash,
        InvalidSource::ContainsLineBreak => ParseError::SourceContainsLineBreak,
    }
}

fn invalid_checksum(error: InvalidChecksum) -> ParseError {
    match error {
        InvalidChecksum::InvalidChecksumAlgo => ParseError::InvalidChecksumAlgorithm,
        InvalidChecksum::InvalidChecksum(algo) => ParseError::InvalidChecksumDigest(algo),
        InvalidChecksum::InvalidFormat => ParseError::InvalidChecksumFormat,
    }
}

impl FromStr for Contents {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse_contents(text)
    }
}

impl TryFrom<&str> for Contents {
    type Error = ParseError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        parse_contents(text)
    }
}

#[derive(Debug, Clone)]
enum DepInfoLine {
    Whitespace,
    TargetRule(Target, Vec<Source>),
    SourceRule(Source),
    EnvDepComment(EnvKey, Option<EnvVal>),
    ChecksumComment(Source, Metadata),
}

fn parse_line(line: &str) -> Result<DepInfoLine, ParseError> {
    if line.chars().all(char::is_whitespace) {
        Ok(DepInfoLine::Whitespace)
    } else if line.starts_with("# env-dep:") {
        let (key, value) = parse_env_dep_comment(line)?;
        Ok(DepInfoLine::EnvDepComment(key, value))
    } else if line.starts_with("# checksum:") {
        let (source, metadata) = parse_checksum_comment(line)?;
        Ok(DepInfoLine::ChecksumComment(source, metadata))
    } else if line.contains(": ") {
        let (target, sources) = parse_target_rule(line)?;
        Ok(DepInfoLine::TargetRule(target, sources))
    } else if line.ends_with(':') {
        let source = parse_source_rule(line)?;
        Ok(DepInfoLine::SourceRule(source))
    } else {
        Err(ParseError::UnknownLine(line.to_owned()))
    }
}

/// Parse the contents of a `.d` dep-info file generated by rustc.
///
/// Expects target rules, source rules, environment comments, and checksum comments in that order.
///
/// # Errors
///
/// Returns an error for malformed, inconsistent, or out-of-order records.
fn parse_contents(text: &str) -> Result<Contents, ParseError> {
    let mut contents = Contents::default();

    let mut lines = text.lines().map(parse_line).peekable();

    // Parse target rules.
    let mut first_target_sources: Option<Vec<Source>> = None;
    while let Some(parsed_line) = lines.peek() {
        match parsed_line {
            Ok(DepInfoLine::TargetRule(_, _)) => {
                let Some(Ok(DepInfoLine::TargetRule(target, sources))) = lines.next() else {
                    unreachable!()
                };
                if let Some(first_target_sources) = &first_target_sources {
                    if &sources != first_target_sources {
                        return Err(ParseError::TargetSourcesMismatch);
                    }
                } else {
                    first_target_sources = Some(sources);
                }
                contents.targets.insert(target);
            }
            Ok(DepInfoLine::Whitespace) => {
                let _ = lines.next();
            }
            Ok(_) => {
                break;
            }
            Err(_) => {
                let Some(Err(invalid_line)) = lines.next() else {
                    unreachable!()
                };
                return Err(invalid_line);
            }
        }
    }

    // Parse source rules.
    let mut sources = vec![];
    while let Some(parsed_line) = lines.peek() {
        match parsed_line {
            Ok(DepInfoLine::SourceRule(_)) => {
                let Some(Ok(DepInfoLine::SourceRule(source))) = lines.next() else {
                    unreachable!()
                };
                sources.push(source);
            }
            Ok(DepInfoLine::Whitespace) => {
                let _ = lines.next();
            }
            Ok(DepInfoLine::TargetRule(target, _)) => {
                return Err(ParseError::MisplacedLine(format!(
                    "target rule: {}",
                    target.as_str()
                )));
            }
            Ok(_) => {
                break;
            }
            Err(_) => {
                let Some(Err(invalid_line)) = lines.next() else {
                    unreachable!()
                };
                return Err(invalid_line);
            }
        }
    }
    if let Some(ref target_sources) = first_target_sources
        && sources != *target_sources
    {
        return Err(ParseError::SourceRulesMismatch);
    }
    contents
        .sources
        .extend(sources.into_iter().map(|source| (source, None)));

    // Parse env-dep comments.
    while let Some(parsed_line) = lines.peek() {
        match parsed_line {
            Ok(DepInfoLine::EnvDepComment(_, _)) => {
                let Some(Ok(DepInfoLine::EnvDepComment(key, opt_value))) = lines.next() else {
                    unreachable!()
                };
                contents.env.insert(key, opt_value);
            }
            Ok(DepInfoLine::Whitespace) => {
                let _ = lines.next();
            }
            Ok(DepInfoLine::TargetRule(target, _)) => {
                return Err(ParseError::MisplacedLine(format!(
                    "target rule: {}",
                    target.as_str()
                )));
            }
            Ok(DepInfoLine::SourceRule(source)) => {
                return Err(ParseError::MisplacedLine(format!(
                    "source rule: {}",
                    source.as_str()
                )));
            }
            Ok(_) => {
                break;
            }
            Err(_) => {
                let Some(Err(invalid_line)) = lines.next() else {
                    unreachable!()
                };
                return Err(invalid_line);
            }
        }
    }

    // Parse checksum comments.
    while let Some(parsed_line) = lines.peek() {
        match parsed_line {
            Ok(DepInfoLine::ChecksumComment(_, _)) => {
                let Some(Ok(DepInfoLine::ChecksumComment(source, metadata))) = lines.next() else {
                    unreachable!()
                };
                let Some(entry) = contents.sources.get_mut(&source) else {
                    return Err(ParseError::ChecksumSource(source.as_str().to_owned()));
                };
                if entry.is_some() {
                    return Err(ParseError::ChecksumSource(source.as_str().to_owned()));
                }
                *entry = Some(metadata);
            }
            Ok(DepInfoLine::Whitespace) => {
                let _ = lines.next();
            }
            Ok(DepInfoLine::TargetRule(target, _)) => {
                return Err(ParseError::MisplacedLine(format!(
                    "target rule: {}",
                    target.as_str()
                )));
            }
            Ok(DepInfoLine::SourceRule(source)) => {
                return Err(ParseError::MisplacedLine(format!(
                    "source rule: {}",
                    source.as_str()
                )));
            }
            Ok(DepInfoLine::EnvDepComment(key, _)) => {
                return Err(ParseError::MisplacedLine(format!(
                    "env-dep comment: {}",
                    key.as_str()
                )));
            }
            Err(_) => {
                let Some(Err(invalid_line)) = lines.next() else {
                    unreachable!()
                };
                return Err(invalid_line);
            }
        }
    }

    Ok(contents)
}

fn parse_target_rule(line: &str) -> Result<(Target, Vec<Source>), ParseError> {
    let (target, paths) = line
        .split_once(": ")
        .ok_or(ParseError::InvalidTargetRuleSyntax)?;
    let target = Target::try_from(target).map_err(|error| match error {
        InvalidTarget::Empty => ParseError::EmptyTarget,
        InvalidTarget::ContainsLineBreak => ParseError::TargetContainsLineBreak,
        InvalidTarget::ContainsRuleSeparator => unreachable!("split at first rule separator"),
    })?;
    let mut sources = Vec::new();
    if !paths.is_empty() {
        let mut tokens = paths.split(' ');
        while let Some(token) = tokens.next() {
            let mut path = token.to_owned();
            while path.ends_with('\\') {
                path.pop();
                path.push(' ');
                path.push_str(tokens.next().ok_or(ParseError::TrailingDependencyEscape)?);
            }
            sources.push(Source::try_from(path).map_err(invalid_source)?);
        }
    }
    Ok((target, sources))
}

fn parse_source_rule(line: &str) -> Result<Source, ParseError> {
    let path = line
        .strip_suffix(':')
        .ok_or(ParseError::InvalidSourceRuleSyntax)?;
    Source::try_from(path.replace("\\ ", " ")).map_err(invalid_source)
}

fn parse_env_dep_comment(line: &str) -> Result<(EnvKey, Option<EnvVal>), ParseError> {
    let rest = line
        .strip_prefix("# env-dep:")
        .ok_or(ParseError::InvalidEnvDepSyntax)?;
    let (key, value) = split_env_dep(rest);
    Ok((
        parse_env(key)?.into(),
        value.map(parse_env).transpose()?.map(Into::into),
    ))
}

fn parse_checksum_comment(line: &str) -> Result<(Source, Metadata), ParseError> {
    let rest = line
        .strip_prefix("# checksum:")
        .ok_or(ParseError::InvalidChecksumSyntax)?;
    let mut parts = rest.splitn(3, ' ');
    let checksum_text = parts.next().unwrap_or_default();
    if checksum_text.matches('=').count() != 1 {
        return Err(ParseError::InvalidChecksumFormat);
    }
    let checksum = checksum_text
        .parse::<Checksum>()
        .map_err(invalid_checksum)?;
    let file_len = parts
        .next()
        .and_then(|s| s.strip_prefix("file_len:"))
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or(ParseError::InvalidChecksumFileLength)?;
    let path = parts.next().ok_or(ParseError::InvalidChecksumSyntax)?;
    let source = Source::try_from(path.replace("\\ ", " ")).map_err(invalid_source)?;
    Ok((source, Metadata { file_len, checksum }))
}

fn split_env_dep(text: &str) -> (&str, Option<&str>) {
    let mut escaped = false;
    for (index, c) in text.char_indices() {
        match c {
            '\\' => escaped = !escaped,
            '=' if !escaped => return (&text[..index], Some(&text[index + 1..])),
            _ => escaped = false,
        }
    }
    (text, None)
}

fn parse_env(text: &str) -> Result<String, ParseError> {
    let mut value = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            value.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => value.push('\\'),
            Some('=') => value.push('='),
            Some('n') => value.push('\n'),
            Some('r') => value.push('\r'),
            Some(c) => return Err(ParseError::InvalidEnvironmentEscape(c)),
            None => return Err(ParseError::UnterminatedEnvironmentEscape),
        }
    }
    Ok(value)
}

impl FromStr for Checksum {
    type Err = InvalidChecksum;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('=');
        let algo = parts
            .next()
            .expect("split always yields a first part")
            .parse::<ChecksumAlgo>()?;
        let Some(checksum) = parts.next() else {
            return Err(InvalidChecksum::InvalidFormat);
        };
        let mut value = [0; 32];
        if hex::decode_to_slice(checksum, &mut value[0..algo.hash_len()]).is_err() {
            return Err(InvalidChecksum::InvalidChecksum(algo));
        }
        Ok(Self { algo, value })
    }
}

impl FromStr for ChecksumAlgo {
    type Err = InvalidChecksum;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sha256" => Ok(Self::Sha256),
            "blake3" => Ok(Self::Blake3),
            _ => Err(InvalidChecksum::InvalidChecksumAlgo),
        }
    }
}

impl TryFrom<String> for Source {
    type Error = InvalidSource;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        if path.is_empty() {
            Err(InvalidSource::Empty)
        } else if path.ends_with('\\') {
            Err(InvalidSource::TrailingBackslash)
        } else if path.contains('\n') || path.contains('\r') {
            Err(InvalidSource::ContainsLineBreak)
        } else {
            Ok(Self(path))
        }
    }
}

impl TryFrom<&str> for Source {
    type Error = InvalidSource;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::try_from(path.to_owned())
    }
}

impl TryFrom<String> for Target {
    type Error = InvalidTarget;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        if path.is_empty() {
            Err(InvalidTarget::Empty)
        } else if path.contains(": ") {
            Err(InvalidTarget::ContainsRuleSeparator)
        } else if path.contains('\n') || path.contains('\r') {
            Err(InvalidTarget::ContainsLineBreak)
        } else {
            Ok(Self(path))
        }
    }
}

impl TryFrom<&str> for Target {
    type Error = InvalidTarget;

    fn try_from(path: &str) -> Result<Self, Self::Error> {
        Self::try_from(path.to_owned())
    }
}

impl<'de> serde::Deserialize<'de> for Checksum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod line_tests {
    use super::*;

    #[test]
    fn parses_line_types_and_escaped_paths() {
        assert!(matches!(parse_line("  "), Ok(DepInfoLine::Whitespace)));
        match parse_line(r"app: dir\ with\ spaces/a.rs").unwrap() {
            DepInfoLine::TargetRule(target, sources) => {
                assert_eq!(target.as_str(), "app");
                assert_eq!(sources[0].as_str(), "dir with spaces/a.rs");
            }
            other => panic!("unexpected line: {other:?}"),
        }
        assert!(matches!(
            parse_line("src.rs:"),
            Ok(DepInfoLine::SourceRule(_))
        ));
        assert!(matches!(
            parse_line("# env-dep:MODE=debug"),
            Ok(DepInfoLine::EnvDepComment(_, Some(_)))
        ));
        assert!(matches!(
            parse_line(
                "# checksum:sha256=ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad file_len:3 src.rs"
            ),
            Ok(DepInfoLine::ChecksumComment(_, _))
        ));
    }

    #[test]
    fn reports_specific_line_errors() {
        assert!(matches!(
            parse_line(r"app: src.rs\"),
            Err(ParseError::TrailingDependencyEscape)
        ));
        assert!(matches!(parse_line(":"), Err(ParseError::EmptySource)));
        assert!(matches!(
            parse_line(r"# env-dep:KEY=bad\t"),
            Err(ParseError::InvalidEnvironmentEscape('t'))
        ));
        assert!(matches!(
            parse_line("# checksum:sha256=invalid file_len:3 src.rs"),
            Err(ParseError::InvalidChecksumDigest(ChecksumAlgo::Sha256))
        ));
        assert!(matches!(
            parse_line("# ordinary comment"),
            Err(ParseError::UnknownLine(_))
        ));
    }
}
