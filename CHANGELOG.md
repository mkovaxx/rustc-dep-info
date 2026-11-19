# Changelog

## 0.3.0

### Breaking changes

- Parsing is now strict: target rules, source rules, environment dependency comments,
  and checksum comments must appear in that order. Empty and whitespace-only lines
  are ignored, and sections may be omitted when consistency rules permit.
- Unrecognized lines, malformed records, and duplicates are rejected instead of
  being ignored or combined. Target rules must list identical source dependencies
  in identical order, with exactly one source rule per dependency. Source rules may
  appear in a different order. Checksums must refer to existing source rules.
- `ParseError` is now a struct with `line`, `column`, and `details` fields. Match on
  `details: ParseErrorKind` instead of matching on `ParseError` variants. Positions
  are one-based; columns count Unicode scalar values. Missing-source errors point
  to the original target dependency.
- `Source::new` and `Target::new` replace their `TryFrom<String>` and `TryFrom<&str>`
  implementations. Constructor errors are now `SourceError` and `TargetError`,
  replacing `InvalidSource` and `InvalidTarget`.
- `Target::new` rejects paths that would print as reserved comments: `# env-dep`,
  `# checksum`, and paths starting with either name followed by `:`. Source paths
  remain valid with these names because their spaces are escaped when printed.
- `EnvKey::new` and `EnvVal::new` replace their `From<String>` and `From<&str>`
  implementations. Constructors accept decoded text and preserve it exactly.
- `Digest::Sha256([u8; 32])` and `Digest::Blake3([u8; 32])` replace `Checksum` and
  `ChecksumAlgo`. `Metadata::checksum` is now `Metadata::digest`. `Digest::value()`
  returns the raw bytes; `Display` retains the `algorithm=hex_digest` format.
- Removed standalone checksum and algorithm parsing, `InvalidChecksum`, and checksum
  Serde serialization and deserialization. Checksums are parsed as part of `Contents`.

### Improvements

- Source-rule membership checks use an ordered index, avoiding quadratic validation
  while preserving dependency order and diagnostic locations.
- Updated `arbitrary` path generation to validate through public constructors and
  avoid reserved target comment prefixes.
- Expanded coverage for complete diagnostics, optional sections, boundary values,
  exact printed output, reserved comment prefixes, and malformed text that must not
  panic or report invalid locations.
- Documented the strict parsing contract in the README and public API documentation.
