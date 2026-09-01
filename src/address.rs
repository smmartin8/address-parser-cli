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
pub fn parse(input: &str) -> Result<Address, ParseError> {
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

    // The city/state/zip line is expected as "City, ST ZIP" — split on the
    // last comma so multi-word city names ("Winston, Salem" is unusual but
    // "Salt Lake City, UT 84101" is not) still work.
    let comma_pos = last_line
        .rfind(',')
        .ok_or_else(|| ParseError::MalformedCityStateZip(last_line.to_string()))?;
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

    let state_upper = state.to_uppercase();
    if !US_STATES.contains(&state_upper.as_str()) {
        return Err(ParseError::UnknownState(state.to_string()));
    }

    if !is_valid_zip(zip) {
        return Err(ParseError::InvalidZip(zip.to_string()));
    }

    Ok(Address {
        street_lines: street_lines.iter().map(|s| normalize_street_line(s)).collect(),
        city: city.to_string(),
        state: state_upper,
        zip: zip.to_string(),
    })
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

fn is_valid_zip(zip: &str) -> bool {
    fn digits_only(s: &str) -> bool {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
    }

    match zip.split_once('-') {
        Some((base, ext)) => base.len() == 5 && digits_only(base) && ext.len() == 4 && digits_only(ext),
        None => zip.len() == 5 && digits_only(zip),
    }
}

impl Address {
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
}
