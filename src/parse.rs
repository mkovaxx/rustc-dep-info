use std::str::FromStr;

use crate::{
    Checksum, ChecksumAlgo, Contents, EnvKey, EnvVal, InvalidChecksum, InvalidSource,
    InvalidTarget, Metadata, Source, Target,
};

/// An error parsing the contents of a rustc dep-info file.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseError {
    /// A dependency path ends with an escape character and has no following token.
    #[error("unterminated escaped dependency path")]
    TrailingDependencyEscape,

    /// An environment variable name or value contains an unknown escape.
    #[error("unknown escape character `{0}`")]
    InvalidEnvironmentEscape(char),

    /// An environment variable name or value ends with an escape character.
    #[error("unterminated escape character")]
    UnterminatedEnvironmentEscape,

    /// A rule contains an invalid path.
    #[error("invalid path: {0}")]
    InvalidPath(String),

    /// Target rules disagree on their source paths.
    #[error("target rules have different sources")]
    TargetSourcesMismatch,

    /// Source rules do not match the target rules.
    #[error("source rules do not match target rules")]
    SourceRulesMismatch,

    /// A checksum references an unknown source or repeats a source.
    #[error("invalid or duplicate checksum source: {0}")]
    ChecksumSource(String),

    /// A checksum algorithm or digest could not be parsed.
    #[error("invalid checksum: {0}")]
    InvalidChecksum(#[from] InvalidChecksum),

    /// A line is malformed or appears in the wrong section.
    #[error("unexpected or malformed line: {0}")]
    UnexpectedLine(String),
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
    TargetRule(TargetRule),
    SourceRule(SourceRule),
    UnknownRule(String),
    EnvDepComment(EnvDepComment),
    ChecksumComment(ChecksumComment),
    UnknownComment(String),
    Unknown(String),
}

/// A target rule is complete if all following conditions hold:
///
/// - `target.is_some()`
/// - `leftover.chars().all(char::is_whitespace)`
#[derive(Debug, Clone)]
struct TargetRule {
    target: Option<Target>,
    sources: Vec<Source>,
    leftover: String,
}

impl TryFrom<TargetRule> for (Target, Vec<Source>) {
    type Error = ParseError;

    fn try_from(value: TargetRule) -> Result<Self, Self::Error> {
        if !value.leftover.chars().all(char::is_whitespace) {
            if value.leftover.ends_with('\\') {
                return Err(ParseError::TrailingDependencyEscape);
            }
            return Err(ParseError::UnexpectedLine(value.leftover));
        }
        let target = value
            .target
            .ok_or_else(|| ParseError::InvalidPath("missing target".into()))?;
        Ok((target, value.sources))
    }
}

/// A source rule is complete if all following conditions hold:
///
/// - `source.is_some()`
/// - `leftover.chars().all(char::is_whitespace)`
#[derive(Debug, Clone)]
struct SourceRule {
    source: Option<Source>,
    leftover: String,
}

impl TryFrom<SourceRule> for Source {
    type Error = ParseError;

    fn try_from(value: SourceRule) -> Result<Self, Self::Error> {
        if !value.leftover.chars().all(char::is_whitespace) {
            return Err(ParseError::UnexpectedLine(value.leftover));
        }
        value
            .source
            .ok_or_else(|| ParseError::InvalidPath("missing source".into()))
    }
}

/// An env-dep comment is complete if all following conditions hold:
///
/// - `key.is_some()`
/// - `leftover.chars().all(char::is_whitespace)`
#[derive(Debug, Clone)]
struct EnvDepComment {
    key: Option<EnvKey>,
    value: Option<EnvVal>,
    leftover: String,
}

impl TryFrom<EnvDepComment> for (EnvKey, Option<EnvVal>) {
    type Error = ParseError;

    fn try_from(value: EnvDepComment) -> Result<Self, Self::Error> {
        if !value.leftover.is_empty() {
            let text = value.leftover.strip_prefix('=').unwrap_or(&value.leftover);
            parse_env(text)?;
            if !value.leftover.chars().all(char::is_whitespace) {
                return Err(ParseError::UnexpectedLine(value.leftover));
            }
        }
        let key = value
            .key
            .ok_or_else(|| ParseError::UnexpectedLine("missing environment key".into()))?;
        Ok((key, value.value))
    }
}

/// A checksum comment is complete if all following conditions hold:
///
/// - `source.is_some()`
/// - `file_len.is_some()`
/// - `algo.is_some()`
/// - `value.len() == REQUIRED_BY_ALGO`
/// - `leftover.chars().all(char::is_whitespace)`
#[derive(Debug, Clone)]
struct ChecksumComment {
    source: Option<Source>,
    file_len: Option<u64>,
    algo: Option<ChecksumAlgo>,
    value: Vec<u8>,
    leftover: String,
}

impl TryFrom<ChecksumComment> for (Source, Metadata) {
    type Error = ParseError;

    fn try_from(value: ChecksumComment) -> Result<Self, Self::Error> {
        let algo = match value.algo {
            Some(algo) => algo,
            None => {
                return Err(ParseError::InvalidChecksum(
                    InvalidChecksum::InvalidChecksumAlgo,
                ));
            }
        };
        if value.value.len() != algo.hash_len() {
            return Err(ParseError::InvalidChecksum(
                InvalidChecksum::InvalidChecksum(algo),
            ));
        }
        if !value.leftover.chars().all(char::is_whitespace) {
            return Err(ParseError::UnexpectedLine(value.leftover));
        }
        let source = value
            .source
            .ok_or_else(|| ParseError::UnexpectedLine("missing checksum source".into()))?;
        let file_len = value
            .file_len
            .ok_or_else(|| ParseError::UnexpectedLine("missing checksum length".into()))?;
        let mut bytes = [0; 32];
        bytes[..value.value.len()].copy_from_slice(&value.value);
        Ok((
            source,
            Metadata {
                file_len,
                checksum: Checksum::new(algo, bytes),
            },
        ))
    }
}

fn parse_line(line: &str) -> DepInfoLine {
    if line.chars().all(char::is_whitespace) {
        DepInfoLine::Whitespace
    } else if let Some(comment) = parse_env_dep_comment(line) {
        DepInfoLine::EnvDepComment(comment)
    } else if let Some(comment) = parse_checksum_comment(line) {
        DepInfoLine::ChecksumComment(comment)
    } else if let Some(rule) = parse_target_rule(line) {
        DepInfoLine::TargetRule(rule)
    } else if let Some(rule) = parse_source_rule(line) {
        DepInfoLine::SourceRule(rule)
    } else if line.starts_with('#') {
        DepInfoLine::UnknownComment(line.to_owned())
    } else if line.contains(':') {
        DepInfoLine::UnknownRule(line.to_owned())
    } else {
        DepInfoLine::Unknown(line.to_owned())
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
            DepInfoLine::TargetRule(_) => {
                let Some(DepInfoLine::TargetRule(target_rule)) = lines.next() else {
                    unreachable!()
                };
                let (target, sources) = target_rule.try_into()?;
                if let Some(first_target_sources) = &first_target_sources {
                    if &sources != first_target_sources {
                        return Err(ParseError::TargetSourcesMismatch);
                    }
                } else {
                    first_target_sources = Some(sources);
                }
                contents.targets.insert(target);
            }
            DepInfoLine::Whitespace => {
                let _ = lines.next();
            }
            DepInfoLine::UnknownRule(line)
            | DepInfoLine::UnknownComment(line)
            | DepInfoLine::Unknown(line) => {
                return Err(ParseError::UnexpectedLine(line.clone()));
            }
            _ => {
                break;
            }
        }
    }

    // Parse source rules.
    let mut sources = vec![];
    while let Some(parsed_line) = lines.peek() {
        match parsed_line {
            DepInfoLine::SourceRule(_) => {
                let Some(DepInfoLine::SourceRule(source_rule)) = lines.next() else {
                    unreachable!()
                };
                let source = source_rule.try_into()?;
                sources.push(source);
            }
            DepInfoLine::Whitespace => {
                let _ = lines.next();
            }
            DepInfoLine::TargetRule(_) => {
                return Err(ParseError::UnexpectedLine(format!("{parsed_line:?}")));
            }
            DepInfoLine::UnknownRule(line)
            | DepInfoLine::UnknownComment(line)
            | DepInfoLine::Unknown(line) => {
                return Err(ParseError::UnexpectedLine(line.clone()));
            }
            _ => {
                break;
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
            DepInfoLine::EnvDepComment(_) => {
                let Some(DepInfoLine::EnvDepComment(env_dep_comment)) = lines.next() else {
                    unreachable!()
                };
                let (key, opt_value) = env_dep_comment.try_into()?;
                contents.env.insert(key, opt_value);
            }
            DepInfoLine::Whitespace => {
                let _ = lines.next();
            }
            DepInfoLine::TargetRule(_) | DepInfoLine::SourceRule(_) => {
                return Err(ParseError::UnexpectedLine(format!("{parsed_line:?}")));
            }
            DepInfoLine::UnknownRule(line)
            | DepInfoLine::UnknownComment(line)
            | DepInfoLine::Unknown(line) => {
                return Err(ParseError::UnexpectedLine(line.clone()));
            }
            _ => {
                break;
            }
        }
    }

    // Parse checksum comments.
    while let Some(parsed_line) = lines.peek() {
        match parsed_line {
            DepInfoLine::ChecksumComment(_) => {
                let Some(DepInfoLine::ChecksumComment(checksum_comment)) = lines.next() else {
                    unreachable!()
                };
                let (source, metadata) = checksum_comment.try_into()?;
                let Some(entry) = contents.sources.get_mut(&source) else {
                    return Err(ParseError::ChecksumSource(source.as_str().to_owned()));
                };
                if entry.is_some() {
                    return Err(ParseError::ChecksumSource(source.as_str().to_owned()));
                }
                *entry = Some(metadata);
            }
            DepInfoLine::Whitespace => {
                let _ = lines.next();
            }
            DepInfoLine::TargetRule(_)
            | DepInfoLine::SourceRule(_)
            | DepInfoLine::EnvDepComment(_) => {
                return Err(ParseError::UnexpectedLine(format!("{parsed_line:?}")));
            }
            DepInfoLine::UnknownRule(line)
            | DepInfoLine::UnknownComment(line)
            | DepInfoLine::Unknown(line) => {
                return Err(ParseError::UnexpectedLine(line.clone()));
            }
        }
    }

    Ok(contents)
}

fn parse_target_rule(line: &str) -> Option<TargetRule> {
    let (target, paths) = line.split_once(": ")?;
    let mut rule = TargetRule {
        target: Target::try_from(target).ok(),
        sources: Vec::new(),
        leftover: String::new(),
    };
    if rule.target.is_none() {
        rule.leftover = line.to_owned();
        return Some(rule);
    }
    let mut rest = paths;
    while !rest.is_empty() {
        let mut end = rest.len();
        let mut escaped = false;
        for (index, c) in rest.char_indices() {
            if c == ' ' && !escaped {
                end = index;
                break;
            }
            escaped = c == '\\';
        }
        let path = &rest[..end];
        match Source::try_from(path.replace("\\ ", " ")) {
            Ok(source) => rule.sources.push(source),
            Err(_) => {
                rule.leftover = rest.to_owned();
                break;
            }
        }
        rest = if end < rest.len() {
            &rest[end + 1..]
        } else {
            ""
        };
    }
    return Some(rule);
}

fn parse_source_rule(line: &str) -> Option<SourceRule> {
    let path = line.strip_suffix(':')?;
    let source = Source::try_from(path.replace("\\ ", " ")).ok();
    let leftover = if source.is_some() {
        String::new()
    } else {
        line.to_owned()
    };
    return Some(SourceRule { source, leftover });
}

fn parse_env_dep_comment(line: &str) -> Option<EnvDepComment> {
    let rest = line.strip_prefix("# env-dep:")?;
    let (key_text, value_text) = split_env_dep(rest);
    let mut comment = EnvDepComment {
        key: None,
        value: None,
        leftover: String::new(),
    };
    match parse_env(key_text) {
        Ok(key) => comment.key = Some(key.into()),
        Err(_) => {
            comment.leftover = rest.to_owned();
            return Some(comment);
        }
    }
    if let Some(value_text) = value_text {
        match parse_env(value_text) {
            Ok(value) => comment.value = Some(value.into()),
            Err(_) => comment.leftover = format!("={value_text}"),
        }
    }
    return Some(comment);
}

fn parse_checksum_comment(line: &str) -> Option<ChecksumComment> {
    let rest = line.strip_prefix("# checksum:")?;
    let mut comment = ChecksumComment {
        source: None,
        file_len: None,
        algo: None,
        value: Vec::new(),
        leftover: String::new(),
    };
    let Some((algo, rest)) = rest.split_once('=') else {
        comment.leftover = rest.to_owned();
        return Some(comment);
    };
    let Ok(algo) = algo.parse::<ChecksumAlgo>() else {
        comment.leftover = format!("{algo}={rest}");
        return Some(comment);
    };
    comment.algo = Some(algo);
    let (digest, rest) = rest.split_once(' ').unwrap_or((rest, ""));
    match hex::decode(digest) {
        Ok(value) => comment.value = value,
        Err(_) => {
            comment.leftover = format!(
                "{digest}{rest_separator}{rest}",
                rest_separator = if rest.is_empty() { "" } else { " " }
            );
            return Some(comment);
        }
    }
    let (length, rest) = rest.split_once(' ').unwrap_or((rest, ""));
    let Some(file_len) = length
        .strip_prefix("file_len:")
        .and_then(|s| s.parse().ok())
    else {
        comment.leftover = format!(
            "{length}{rest_separator}{rest}",
            rest_separator = if rest.is_empty() { "" } else { " " }
        );
        return Some(comment);
    };
    comment.file_len = Some(file_len);
    match Source::try_from(rest.replace("\\ ", " ")) {
        Ok(source) => comment.source = Some(source),
        Err(_) => comment.leftover = rest.to_owned(),
    }
    return Some(comment);
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
