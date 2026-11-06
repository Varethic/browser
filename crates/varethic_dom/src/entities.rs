//! Minimal character-reference decoding for the HTML tokenizer.
//!
//! This is a deliberately small subset of the WHATWG named character reference
//! table plus full numeric reference support. It is NOT spec-complete: only the
//! ~40 most common named references are recognized. When a reference is not
//! recognized, the input is left untouched (matching the spec's behavior of
//! emitting a literal `&`). Tracked in docs/KNOWN_ISSUES.md.

/// Decode every valid character reference in `s`, leaving unrecognized `&`
/// sequences untouched.
pub fn decode_entities(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '&' {
            if let Some((text, consumed)) = decode_char_ref(&chars, i) {
                out.push_str(&text);
                i += consumed;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Attempt to decode a character reference starting at `chars[i]` (which must be
/// `'&'`). Returns the decoded text and how many characters were consumed
/// (including the leading `&`), or `None` if this is not a valid reference.
pub fn decode_char_ref(chars: &[char], i: usize) -> Option<(String, usize)> {
    if chars.get(i) != Some(&'&') {
        return None;
    }
    let rest = &chars[i + 1..];
    if rest.is_empty() {
        return None;
    }

    // Numeric references.
    if rest[0] == '#' {
        let (radix, digits_start) = if rest.get(1) == Some(&'x') || rest.get(1) == Some(&'X') {
            (16u32, 2)
        } else {
            (10u32, 1)
        };
        let mut j = digits_start;
        let mut value: u32 = 0;
        let mut any = false;
        while let Some(c) = rest.get(j) {
            match c.to_digit(radix) {
                Some(d) => {
                    value = value.saturating_mul(radix).saturating_add(d);
                    any = true;
                    j += 1;
                }
                None => break,
            }
        }
        if !any {
            return None;
        }
        let consumed = 1 + j + if rest.get(j) == Some(&';') { 1 } else { 0 };
        // Map out-of-range / surrogate values to U+FFFD, as the spec requires.
        let decoded = char::from_u32(value).unwrap_or('\u{FFFD}');
        return Some((decoded.to_string(), consumed));
    }

    // Named references: longest match from a small table, optional trailing ';'.
    let max = rest.len().min(32);
    for end in (1..=max).rev() {
        let name: String = rest[..end].iter().collect();
        let name_lower = name.to_ascii_lowercase();
        if let Some(decoded) = named(&name_lower) {
            let consumed = 1 + end;
            return Some((decoded.to_string(), consumed));
        }
    }
    None
}

/// A small subset of the WHATWG named character reference table.
fn named(name: &str) -> Option<char> {
    Some(match name {
        // Note: names are matched with trailing ';' stripped by the caller's
        // longest-match loop, so accept both `amp` and `amp;`.
        s if s == "amp" || s == "amp;" => '&',
        s if s == "lt" || s == "lt;" => '<',
        s if s == "gt" || s == "gt;" => '>',
        s if s == "quot" || s == "quot;" => '"',
        s if s == "apos" || s == "apos;" => '\'',
        s if s == "nbsp" || s == "nbsp;" => '\u{00A0}',
        s if s == "copy" || s == "copy;" => '\u{00A9}',
        s if s == "reg" || s == "reg;" => '\u{00AE}',
        s if s == "trade" || s == "trade;" => '\u{2122}',
        s if s == "hellip" || s == "hellip;" => '\u{2026}',
        s if s == "mdash" || s == "mdash;" => '\u{2014}',
        s if s == "ndash" || s == "ndash;" => '\u{2013}',
        s if s == "lsquo" || s == "lsquo;" => '\u{2018}',
        s if s == "rsquo" || s == "rsquo;" => '\u{2019}',
        s if s == "ldquo" || s == "ldquo;" => '\u{201C}',
        s if s == "rdquo" || s == "rdquo;" => '\u{201D}',
        s if s == "bull" || s == "bull;" => '\u{2022}',
        s if s == "middot" || s == "middot;" => '\u{00B7}',
        s if s == "times" || s == "times;" => '\u{00D7}',
        s if s == "divide" || s == "divide;" => '\u{00F7}',
        s if s == "deg" || s == "deg;" => '\u{00B0}',
        s if s == "plusmn" || s == "plusmn;" => '\u{00B1}',
        s if s == "frac12" || s == "frac12;" => '\u{00BD}',
        s if s == "sect" || s == "sect;" => '\u{00A7}',
        s if s == "para" || s == "para;" => '\u{00B6}',
        s if s == "euro" || s == "euro;" => '\u{20AC}',
        s if s == "pound" || s == "pound;" => '\u{00A3}',
        s if s == "cent" || s == "cent;" => '\u{00A2}',
        s if s == "yen" || s == "yen;" => '\u{00A5}',
        s if s == "laquo" || s == "laquo;" => '\u{00AB}',
        s if s == "raquo" || s == "raquo;" => '\u{00BB}',
        s if s == "larr" || s == "larr;" => '\u{2190}',
        s if s == "rarr" || s == "rarr;" => '\u{2192}',
        s if s == "uarr" || s == "uarr;" => '\u{2191}',
        s if s == "darr" || s == "darr;" => '\u{2193}',
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '&' {
                if let Some((text, consumed)) = decode_char_ref(&chars, i) {
                    out.push_str(&text);
                    i += consumed;
                    continue;
                }
            }
            out.push(chars[i]);
            i += 1;
        }
        out
    }

    #[test]
    fn numeric_decimal_and_hex() {
        assert_eq!(decode("&#65;&#66;"), "AB");
        assert_eq!(decode("&#x41;&#X42;"), "AB");
        assert_eq!(decode("a&#65"), "aA"); // missing semicolon still decodes
    }

    #[test]
    fn named_references() {
        assert_eq!(decode("a &amp; b"), "a & b");
        assert_eq!(decode("&lt;tag&gt;"), "<tag>");
        assert_eq!(decode("&copy;2026"), "\u{00A9}2026");
    }

    #[test]
    fn unknown_reference_left_alone() {
        assert_eq!(decode("a &notreal; b"), "a &notreal; b");
        assert_eq!(decode("AT&T"), "AT&T");
    }

    #[test]
    fn out_of_range_numeric_is_replacement_char() {
        assert_eq!(decode("&#x110000;"), "\u{FFFD}");
    }
}
