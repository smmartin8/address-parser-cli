use std::fmt;

/// Two-letter USPS codes: 50 states, DC, and the territories that get their
/// own code (Puerto Rico, Virgin Islands, Guam, American Samoa, N. Mariana Is.)
pub const US_STATES: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA",
    "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
    "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT",
    "VA", "WA", "WV", "WI", "WY", "DC", "PR", "VI", "GU", "AS", "MP",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    pub street_lines: Vec<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
}

/// Strict rejects anything that doesn't match the documented input shape
/// exactly. Lenient accepts a couple of common real-world sloppiness cases
/// (a missing comma before the state, a zip that lost a leading zero) that
/// are unambiguous to recover from even though they're not well-formed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationMode {
    #[default]
    Strict,
    Lenient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Empty,
    MissingStreet,
    MissingCityStateZip,
    MalformedCityStateZip(String),
    UnknownState(String),
    InvalidZip(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "input is empty"),
            ParseError::MissingStreet => write!(f, "missing street line"),
            ParseError::MissingCityStateZip => write!(f, "missing city/state/zip line"),
            ParseError::MalformedCityStateZip(line) => {
                write!(f, "could not parse city/state/zip line: {line:?}")
            }
            ParseError::UnknownState(code) => write!(f, "unrecognized state code: {code:?}"),
            ParseError::InvalidZip(zip) => write!(f, "invalid zip code: {zip:?}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses a US-style postal address:
///
///   123 Main St
///   Apt 4B                     (optional extra street lines)
///   Springfield, IL 62704
///
/// The last non-blank line must be "City, ST ZIP" or "City, ST ZIP-XXXX".
/// Everything above it is treated as street address lines.
///
/// Equivalent to `parse_with_mode(input, ValidationMode::Strict)`.
pub fn parse(input: &str) -> Result<Address, ParseError> {
    parse_with_mode(input, ValidationMode::Strict)
}

/// Parses zero or more addresses from a single input, where each address is
/// separated from the next by one or more blank lines. A single address with
/// no blank line anywhere behaves exactly like [`parse`] would, wrapped in a
/// one-element vec.
///
/// Equivalent to `parse_all_with_mode(input, ValidationMode::Strict)`.
pub fn parse_all(input: &str) -> Vec<Result<Address, ParseError>> {
    parse_all_with_mode(input, ValidationMode::Strict)
}

/// Same as [`parse_all`], but parses each address block with the given
/// [`ValidationMode`]. One block failing to parse doesn't stop the rest —
/// the caller gets a result per block, in input order, so a batch job can
/// report which entries were bad instead of aborting on the first one.
pub fn parse_all_with_mode(input: &str, mode: ValidationMode) -> Vec<Result<Address, ParseError>> {
    split_into_blocks(input)
        .into_iter()
        .map(|block| parse_with_mode(&block, mode))
        .collect()
}

/// Groups input lines into blocks of consecutive non-blank lines, treating
/// one or more blank lines as a separator between addresses.
fn split_into_blocks(input: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in input.lines() {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

/// Same as [`parse`], but in [`ValidationMode::Lenient`] recovers from a
/// couple of common formatting slips instead of rejecting them outright:
/// a missing comma before the state (falls back to splitting the line on
/// whitespace), and a zip that's short a leading zero (padded back to 5
/// digits). Everything else — unknown state codes, garbage input — is
/// still an error in both modes.
pub fn parse_with_mode(input: &str, mode: ValidationMode) -> Result<Address, ParseError> {
    let lines: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(ParseError::Empty);
    }
    if lines.len() < 2 {
        return Err(ParseError::MissingCityStateZip);
    }

    let (street_lines, last) = lines.split_at(lines.len() - 1);
    let last_line = last[0];

    if street_lines.is_empty() {
        return Err(ParseError::MissingStreet);
    }

    let (city, state, zip) = split_city_state_zip(last_line, mode)?;

    let state_upper = state.to_uppercase();
    if !US_STATES.contains(&state_upper.as_str()) {
        return Err(ParseError::UnknownState(state.to_string()));
    }

    let zip = normalize_zip(zip, mode)
        .ok_or_else(|| ParseError::InvalidZip(zip.to_string()))?;

    Ok(Address {
        street_lines: street_lines.iter().map(|s| normalize_street_line(s)).collect(),
        city: city.to_string(),
        state: state_upper,
        zip,
    })
}

/// Splits the city/state/zip line into its three parts. Strict requires the
/// standard "City, ST ZIP" shape with a comma before the state. Lenient
/// falls back to treating the last two whitespace-separated tokens as state
/// and zip when there's no comma at all.
fn split_city_state_zip<'a>(
    last_line: &'a str,
    mode: ValidationMode,
) -> Result<(&'a str, &'a str, &'a str), ParseError> {
    // The city/state/zip line is expected as "City, ST ZIP" — split on the
    // last comma so multi-word city names ("Winston, Salem" is unusual but
    // "Salt Lake City, UT 84101" is not) still work.
    let comma_pos = match last_line.rfind(',') {
        Some(pos) => pos,
        None if mode == ValidationMode::Lenient => {
            return split_city_state_zip_without_comma(last_line);
        }
        None => return Err(ParseError::MalformedCityStateZip(last_line.to_string())),
    };
    let city_part = &last_line[..comma_pos];
    let rest = last_line[comma_pos + 1..].trim();

    let mut rest_parts = rest.split_whitespace();
    let state = rest_parts
        .next()
        .ok_or_else(|| ParseError::MalformedCityStateZip(last_line.to_string()))?;
    let zip = rest_parts
        .next()
        .ok_or_else(|| ParseError::MalformedCityStateZip(last_line.to_string()))?;
    if rest_parts.next().is_some() {
        return Err(ParseError::MalformedCityStateZip(last_line.to_string()));
    }

    let city = city_part.trim();
    if city.is_empty() {
        return Err(ParseError::MalformedCityStateZip(last_line.to_string()));
    }

    Ok((city, state, zip))
}

fn split_city_state_zip_without_comma(
    last_line: &str,
) -> Result<(&str, &str, &str), ParseError> {
    // Without a comma to anchor on, take the state and zip as the last two
    // whitespace tokens and everything before that as the city. This means
    // internal multi-space runs in the city name get collapsed, but that's
    // an acceptable tradeoff for a format that wasn't well-formed to begin
    // with.
    let zip_start = last_line
        .rfind(char::is_whitespace)
        .map(|i| i + 1)
        .unwrap_or(0);
    let (before_zip, zip) = (&last_line[..zip_start], &last_line[zip_start..]);
    let before_zip = before_zip.trim_end();

    let last_city_end = before_zip.rfind(char::is_whitespace);
    let (city, state) = match last_city_end {
        Some(i) => (before_zip[..i].trim_end(), before_zip[i + 1..].trim()),
        None => return Err(ParseError::MalformedCityStateZip(last_line.to_string())),
    };

    if city.is_empty() || state.is_empty() || zip.is_empty() {
        return Err(ParseError::MalformedCityStateZip(last_line.to_string()));
    }

    Ok((city, state, zip))
}

/// Validates a zip and, in lenient mode, repairs one common data-entry
/// problem: a zip that lost a leading zero (e.g. a New England zip like
/// "02101" round-tripped through a spreadsheet as the number 2101).
fn normalize_zip(zip: &str, mode: ValidationMode) -> Option<String> {
    if is_valid_zip(zip) {
        return Some(zip.to_string());
    }
    if mode == ValidationMode::Lenient {
        let (base, ext) = match zip.split_once('-') {
            Some((base, ext)) => (base, Some(ext)),
            None => (zip, None),
        };
        if !base.is_empty() && base.len() <= 5 && digits_only(base) {
            let padded_base = format!("{base:0>5}");
            let candidate = match ext {
                Some(ext) => format!("{padded_base}-{ext}"),
                None => padded_base,
            };
            if is_valid_zip(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// USPS Publication 28 street suffix abbreviations, keyed by every spelling
/// variant (full word or common alternate abbreviation) that should map to
/// it. Lookup is case-insensitive.
const STREET_SUFFIXES: &[(&str, &str)] = &[
    ("street", "St"), ("str", "St"), ("strt", "St"), ("st", "St"),
    ("avenue", "Ave"), ("aven", "Ave"), ("avn", "Ave"), ("av", "Ave"), ("ave", "Ave"),
    ("boulevard", "Blvd"), ("boul", "Blvd"), ("boulv", "Blvd"), ("blvd", "Blvd"),
    ("drive", "Dr"), ("driv", "Dr"), ("drv", "Dr"), ("dr", "Dr"),
    ("court", "Ct"), ("crt", "Ct"), ("ct", "Ct"),
    ("lane", "Ln"), ("ln", "Ln"),
    ("road", "Rd"), ("rd", "Rd"),
    ("place", "Pl"), ("pl", "Pl"),
    ("terrace", "Ter"), ("terr", "Ter"), ("ter", "Ter"),
    ("circle", "Cir"), ("circ", "Cir"), ("crcl", "Cir"), ("cir", "Cir"),
    ("way", "Way"), ("wy", "Way"),
    ("parkway", "Pkwy"), ("pkwy", "Pkwy"), ("pky", "Pkwy"),
    ("highway", "Hwy"), ("hwy", "Hwy"), ("hiwy", "Hwy"),
    ("trail", "Trl"), ("trl", "Trl"),
    ("square", "Sq"), ("sq", "Sq"),
    ("loop", "Loop"), ("lp", "Loop"),
    ("alley", "Aly"), ("aly", "Aly"),
    ("crossing", "Xing"), ("xing", "Xing"),
    ("extension", "Ext"), ("ext", "Ext"),
    ("junction", "Jct"), ("jct", "Jct"),
    ("turnpike", "Tpke"), ("tpke", "Tpke"),
    ("point", "Pt"), ("pt", "Pt"),
    ("ridge", "Rdg"), ("rdg", "Rdg"),
];

/// Rewrites recognizable street suffix words to their canonical USPS
/// abbreviation, word by word, so it works whether the suffix sits mid-line
/// ("100 Main Street Suite 4") or at the end ("100 Main Street"). Spacing
/// and punctuation attached to the word (a trailing comma, say) are kept.
fn normalize_street_line(line: &str) -> String {
    line.split(' ')
        .map(normalize_suffix_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_suffix_word(word: &str) -> String {
    let trimmed = word.trim_end_matches(|c: char| c == ',' || c == '.');
    let trailing = &word[trimmed.len()..];
    let lower = trimmed.to_lowercase();
    match STREET_SUFFIXES.iter().find(|(variant, _)| *variant == lower) {
        Some((_, canonical)) => format!("{canonical}{trailing}"),
        None => word.to_string(),
    }
}

fn digits_only(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

fn is_valid_zip(zip: &str) -> bool {
    match zip.split_once('-') {
        Some((base, ext)) => base.len() == 5 && digits_only(base) && ext.len() == 4 && digits_only(ext),
        None => zip.len() == 5 && digits_only(zip),
    }
}

impl Address {
    /// Drops the +4 extension, if any, leaving the base 5-digit zip. A
    /// no-op for an address that was already 5 digits.
    pub fn truncate_zip_to_five(&mut self) {
        if let Some((base, _)) = self.zip.split_once('-') {
            self.zip = base.to_string();
        }
    }

    /// Canonical human-readable rendering: one street line per line, state
    /// uppercased, city/state/zip collapsed onto a single trailing line.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        for line in &self.street_lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str(&format!("{}, {} {}", self.city, self.state, self.zip));
        out
    }

    pub fn to_json(&self) -> String {
        let street = self
            .street_lines
            .iter()
            .map(|l| json_string(l))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"street_lines\":[{}],\"city\":{},\"state\":{},\"zip\":{}}}",
            street,
            json_string(&self.city),
            json_string(&self.state),
            json_string(&self.zip),
        )
    }
}

impl ParseError {
    pub fn to_json(&self) -> String {
        format!("{{\"error\":{}}}", json_string(&self.to_string()))
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_address() {
        let addr = parse("123 Main St\nSpringfield, IL 62704").unwrap();
        assert_eq!(addr.street_lines, vec!["123 Main St"]);
        assert_eq!(addr.city, "Springfield");
        assert_eq!(addr.state, "IL");
        assert_eq!(addr.zip, "62704");
    }

    #[test]
    fn parses_zip_plus_four() {
        let addr = parse("1 Infinite Loop\nCupertino, CA 95014-2083").unwrap();
        assert_eq!(addr.zip, "95014-2083");
    }

    #[test]
    fn rejects_unknown_state() {
        let err = parse("1 Main St\nSpringfield, ZZ 62704").unwrap_err();
        assert_eq!(err, ParseError::UnknownState("ZZ".to_string()));
    }

    #[test]
    fn rejects_bad_zip() {
        let err = parse("1 Main St\nSpringfield, IL 6270").unwrap_err();
        assert_eq!(err, ParseError::InvalidZip("6270".to_string()));
    }

    #[test]
    fn rejects_missing_city_line() {
        let err = parse("123 Main St").unwrap_err();
        assert_eq!(err, ParseError::MissingCityStateZip);
    }

    #[test]
    fn normalizes_full_suffix_to_abbreviation() {
        let addr = parse("123 Main Street\nSpringfield, IL 62704").unwrap();
        assert_eq!(addr.street_lines, vec!["123 Main St"]);
    }

    #[test]
    fn normalizes_suffix_mid_line() {
        let addr = parse("100 Main Street Suite 4\nSpringfield, IL 62704").unwrap();
        assert_eq!(addr.street_lines, vec!["100 Main St Suite 4"]);
    }

    #[test]
    fn leaves_already_abbreviated_suffix_alone() {
        let addr = parse("1 Infinite Loop\nCupertino, CA 95014").unwrap();
        assert_eq!(addr.street_lines, vec!["1 Infinite Loop"]);
    }

    #[test]
    fn normalizes_suffix_case_insensitively_and_keeps_trailing_comma() {
        let addr = parse("42 Wallaby avenue,\nSydney, NY 10001").unwrap();
        assert_eq!(addr.street_lines, vec!["42 Wallaby Ave,"]);
    }

    #[test]
    fn does_not_touch_unrelated_words() {
        let addr = parse("500 Elm Boulevard\nApt 4B\nSpringfield, IL 62704").unwrap();
        assert_eq!(addr.street_lines, vec!["500 Elm Blvd", "Apt 4B"]);
    }

    #[test]
    fn strict_rejects_missing_comma() {
        let err = parse_with_mode("1 Main St\nSpringfield IL 62704", ValidationMode::Strict)
            .unwrap_err();
        assert_eq!(
            err,
            ParseError::MalformedCityStateZip("Springfield IL 62704".to_string())
        );
    }

    #[test]
    fn lenient_recovers_missing_comma() {
        let addr = parse_with_mode("1 Main St\nSpringfield IL 62704", ValidationMode::Lenient)
            .unwrap();
        assert_eq!(addr.city, "Springfield");
        assert_eq!(addr.state, "IL");
        assert_eq!(addr.zip, "62704");
    }

    #[test]
    fn lenient_recovers_missing_comma_with_multi_word_city() {
        let addr =
            parse_with_mode("1 Main St\nSalt Lake City UT 84101", ValidationMode::Lenient)
                .unwrap();
        assert_eq!(addr.city, "Salt Lake City");
        assert_eq!(addr.state, "UT");
        assert_eq!(addr.zip, "84101");
    }

    #[test]
    fn strict_rejects_short_zip() {
        let err = parse_with_mode("1 Main St\nBoston, MA 2101", ValidationMode::Strict)
            .unwrap_err();
        assert_eq!(err, ParseError::InvalidZip("2101".to_string()));
    }

    #[test]
    fn lenient_pads_short_zip_with_leading_zero() {
        let addr = parse_with_mode("1 Main St\nBoston, MA 2101", ValidationMode::Lenient)
            .unwrap();
        assert_eq!(addr.zip, "02101");
    }

    #[test]
    fn lenient_pads_short_zip_plus_four() {
        let addr = parse_with_mode("1 Main St\nBoston, MA 2101-0001", ValidationMode::Lenient)
            .unwrap();
        assert_eq!(addr.zip, "02101-0001");
    }

    #[test]
    fn lenient_still_rejects_unknown_state() {
        let err = parse_with_mode("1 Main St\nSpringfield ZZ 62704", ValidationMode::Lenient)
            .unwrap_err();
        assert_eq!(err, ParseError::UnknownState("ZZ".to_string()));
    }

    #[test]
    fn lenient_still_rejects_garbage_zip() {
        let err = parse_with_mode("1 Main St\nBoston, MA abcde", ValidationMode::Lenient)
            .unwrap_err();
        assert_eq!(err, ParseError::InvalidZip("abcde".to_string()));
    }

    #[test]
    fn parse_all_splits_on_blank_lines() {
        let input = "123 Main St\nSpringfield, IL 62704\n\n1 Infinite Loop\nCupertino, CA 95014";
        let results = parse_all(input);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().city, "Springfield");
        assert_eq!(results[1].as_ref().unwrap().city, "Cupertino");
    }

    #[test]
    fn parse_all_treats_multiple_blank_lines_as_one_separator() {
        let input = "123 Main St\nSpringfield, IL 62704\n\n\n\n1 Infinite Loop\nCupertino, CA 95014";
        let results = parse_all(input);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn parse_all_single_address_matches_parse() {
        let input = "123 Main St\nSpringfield, IL 62704";
        let results = parse_all(input);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], parse(input));
    }

    #[test]
    fn parse_all_empty_input_yields_no_blocks() {
        assert_eq!(parse_all("").len(), 0);
        assert_eq!(parse_all("\n\n\n").len(), 0);
    }

    #[test]
    fn parse_all_reports_error_without_aborting_the_batch() {
        let input = "123 Main St\nSpringfield, ZZ 62704\n\n1 Infinite Loop\nCupertino, CA 95014";
        let results = parse_all(input);
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0],
            Err(ParseError::UnknownState("ZZ".to_string()))
        );
        assert_eq!(results[1].as_ref().unwrap().city, "Cupertino");
    }

    #[test]
    fn truncate_zip_to_five_drops_extension() {
        let mut addr = parse("1 Infinite Loop\nCupertino, CA 95014-2083").unwrap();
        addr.truncate_zip_to_five();
        assert_eq!(addr.zip, "95014");
    }

    #[test]
    fn truncate_zip_to_five_is_a_no_op_on_plain_zip() {
        let mut addr = parse("123 Main St\nSpringfield, IL 62704").unwrap();
        addr.truncate_zip_to_five();
        assert_eq!(addr.zip, "62704");
    }

    #[test]
    fn parse_all_with_mode_applies_mode_to_every_block() {
        let input = "1 Main St\nSpringfield IL 62704\n\n1 Infinite Loop\nCupertino CA 95014";
        let results = parse_all_with_mode(input, ValidationMode::Lenient);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().city, "Springfield");
        assert_eq!(results[1].as_ref().unwrap().city, "Cupertino");
    }
}
