use std::fmt;

use crate::address::json_string;

/// Canada's provinces and territories, by their standard two-letter
/// Canada Post abbreviation.
pub const CA_PROVINCES: &[&str] = &[
    "AB", "BC", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "QC", "SK", "YT",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanadianAddress {
    pub street_lines: Vec<String>,
    pub city: String,
    pub province: String,
    pub postal_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaParseError {
    Empty,
    MissingStreet,
    MissingCityProvincePostal,
    MalformedCityProvincePostal(String),
    UnknownProvince(String),
    InvalidPostalCode(String),
}

impl fmt::Display for CaParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaParseError::Empty => write!(f, "input is empty"),
            CaParseError::MissingStreet => write!(f, "missing street line"),
            CaParseError::MissingCityProvincePostal => {
                write!(f, "missing city/province/postal code line")
            }
            CaParseError::MalformedCityProvincePostal(line) => {
                write!(f, "could not parse city/province/postal code line: {line:?}")
            }
            CaParseError::UnknownProvince(code) => write!(f, "unrecognized province code: {code:?}"),
            CaParseError::InvalidPostalCode(code) => write!(f, "invalid postal code: {code:?}"),
        }
    }
}

impl std::error::Error for CaParseError {}

/// Parses a Canadian postal address:
///
///   123 Main St
///   Apt 4B                     (optional extra street lines)
///   Toronto, ON M5V 2T6
///
/// The last non-blank line must be "City, PR A1A 1A1", with or without
/// the internal space in the postal code. Everything above it is treated
/// as street lines, same as the US format. There's no lenient mode here
/// yet — this is the first cut at a second country, kept as close to the
/// US parser's shape as the format allows.
pub fn parse_ca(input: &str) -> Result<CanadianAddress, CaParseError> {
    let lines: Vec<&str> = input
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();

    if lines.is_empty() {
        return Err(CaParseError::Empty);
    }
    if lines.len() < 2 {
        return Err(CaParseError::MissingCityProvincePostal);
    }

    let (street_lines, last) = lines.split_at(lines.len() - 1);
    let last_line = last[0];

    if street_lines.is_empty() {
        return Err(CaParseError::MissingStreet);
    }

    let (city, province, postal_raw) = split_city_province_postal(last_line)?;

    let province_upper = province.to_uppercase();
    if !CA_PROVINCES.contains(&province_upper.as_str()) {
        return Err(CaParseError::UnknownProvince(province.to_string()));
    }

    let postal_code = normalize_postal_code(postal_raw)
        .ok_or_else(|| CaParseError::InvalidPostalCode(postal_raw.to_string()))?;

    Ok(CanadianAddress {
        street_lines: street_lines.iter().map(|s| s.to_string()).collect(),
        city: city.to_string(),
        province: province_upper,
        postal_code,
    })
}

/// Splits the city/province/postal line on the last comma (same rationale
/// as the US parser: a multi-word city shouldn't confuse the split), then
/// takes the first whitespace-separated token after it as the province and
/// the rest as the postal code, since the postal code may or may not have
/// its own internal space ("M5V 2T6" vs "M5V2T6").
fn split_city_province_postal(last_line: &str) -> Result<(&str, &str, &str), CaParseError> {
    let comma_pos = last_line
        .rfind(',')
        .ok_or_else(|| CaParseError::MalformedCityProvincePostal(last_line.to_string()))?;
    let city = last_line[..comma_pos].trim();
    let rest = last_line[comma_pos + 1..].trim();

    let mut rest_parts = rest.splitn(2, char::is_whitespace);
    let province = rest_parts
        .next()
        .ok_or_else(|| CaParseError::MalformedCityProvincePostal(last_line.to_string()))?;
    let postal_raw = rest_parts
        .next()
        .map(str::trim)
        .ok_or_else(|| CaParseError::MalformedCityProvincePostal(last_line.to_string()))?;

    if city.is_empty() || province.is_empty() || postal_raw.is_empty() {
        return Err(CaParseError::MalformedCityProvincePostal(last_line.to_string()));
    }

    Ok((city, province, postal_raw))
}

/// Validates and normalizes a Canadian postal code to "A1A 1A1" form,
/// accepting input with or without the internal space, case-insensitive.
/// Like the US zip check, this validates shape only (letter-digit-letter-
/// digit-letter-digit) — not that the code corresponds to a real location.
fn normalize_postal_code(code: &str) -> Option<String> {
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let chars: Vec<char> = compact.chars().collect();
    if chars.len() != 6 {
        return None;
    }
    let shape_ok = chars[0].is_ascii_alphabetic()
        && chars[1].is_ascii_digit()
        && chars[2].is_ascii_alphabetic()
        && chars[3].is_ascii_digit()
        && chars[4].is_ascii_alphabetic()
        && chars[5].is_ascii_digit();
    if !shape_ok {
        return None;
    }
    let upper = compact.to_uppercase();
    Some(format!("{} {}", &upper[0..3], &upper[3..6]))
}

impl CanadianAddress {
    /// Canonical human-readable rendering, same layout as the US address:
    /// one street line per line, then "City, PR A1A 1A1" on the last line.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        for line in &self.street_lines {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str(&format!("{}, {} {}", self.city, self.province, self.postal_code));
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
            "{{\"street_lines\":[{}],\"city\":{},\"province\":{},\"postal_code\":{}}}",
            street,
            json_string(&self.city),
            json_string(&self.province),
            json_string(&self.postal_code),
        )
    }
}

impl CaParseError {
    pub fn to_json(&self) -> String {
        format!("{{\"error\":{}}}", json_string(&self.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_address() {
        let addr = parse_ca("123 Main St\nToronto, ON M5V 2T6").unwrap();
        assert_eq!(addr.street_lines, vec!["123 Main St"]);
        assert_eq!(addr.city, "Toronto");
        assert_eq!(addr.province, "ON");
        assert_eq!(addr.postal_code, "M5V 2T6");
    }

    #[test]
    fn normalizes_postal_code_without_space() {
        let addr = parse_ca("123 Main St\nToronto, ON M5V2T6").unwrap();
        assert_eq!(addr.postal_code, "M5V 2T6");
    }

    #[test]
    fn normalizes_postal_code_case() {
        let addr = parse_ca("123 Main St\nToronto, on m5v 2t6").unwrap();
        assert_eq!(addr.province, "ON");
        assert_eq!(addr.postal_code, "M5V 2T6");
    }

    #[test]
    fn keeps_multiple_street_lines() {
        let addr = parse_ca("123 Main St\nUnit 4\nToronto, ON M5V 2T6").unwrap();
        assert_eq!(addr.street_lines, vec!["123 Main St", "Unit 4"]);
    }

    #[test]
    fn rejects_unknown_province() {
        let err = parse_ca("123 Main St\nToronto, XX M5V 2T6").unwrap_err();
        assert_eq!(err, CaParseError::UnknownProvince("XX".to_string()));
    }

    #[test]
    fn rejects_malformed_postal_code() {
        let err = parse_ca("123 Main St\nToronto, ON 12345").unwrap_err();
        assert_eq!(err, CaParseError::InvalidPostalCode("12345".to_string()));
    }

    #[test]
    fn rejects_missing_comma() {
        let err = parse_ca("123 Main St\nToronto ON M5V 2T6").unwrap_err();
        assert_eq!(
            err,
            CaParseError::MalformedCityProvincePostal("Toronto ON M5V 2T6".to_string())
        );
    }

    #[test]
    fn rejects_missing_city_line() {
        let err = parse_ca("123 Main St").unwrap_err();
        assert_eq!(err, CaParseError::MissingCityProvincePostal);
    }

    #[test]
    fn rejects_empty_input() {
        let err = parse_ca("").unwrap_err();
        assert_eq!(err, CaParseError::Empty);
    }

    #[test]
    fn to_pretty_matches_input_shape() {
        let addr = parse_ca("123 Main St\nToronto, ON M5V 2T6").unwrap();
        assert_eq!(addr.to_pretty(), "123 Main St\nToronto, ON M5V 2T6");
    }

    #[test]
    fn to_json_renders_fields() {
        let addr = parse_ca("123 Main St\nToronto, ON M5V 2T6").unwrap();
        assert_eq!(
            addr.to_json(),
            "{\"street_lines\":[\"123 Main St\"],\"city\":\"Toronto\",\"province\":\"ON\",\"postal_code\":\"M5V 2T6\"}"
        );
    }
}
