use std::ops::Range;

use crate::scanner::Scanner;

/// Scan for URLs starting from the trigger character ":", requires "://".
///
/// Based on RFC 3986.
pub struct UrlScanner {
    pub trigger: char,
}

impl Scanner for UrlScanner {
    fn scan(&self, s: &str, trigger_index: usize) -> Option<Range<usize>> {
        let (proto, offset) = match self.trigger {
            '/' => ("//", 2),
            _ => ("://", 3),
        };
        let after_slash_slash = trigger_index + offset;
        // Need at least one character for scheme, and one after '//'
        if after_slash_slash < s.len() && s[trigger_index..].starts_with(proto) {
            if let Some(start) = self.find_start(&s[0..trigger_index]) {
                if let Some(end) = self.find_end(&s[after_slash_slash..]) {
                    let range = Range {
                        start,
                        end: after_slash_slash + end,
                    };
                    return Some(range);
                }
            }
        }
        None
    }
}

impl UrlScanner {
    // See "scheme" in RFC 3986
    fn find_start(&self, s: &str) -> Option<usize> {
        // Match protocol relative URLs (`//example.org`)
        // See https://stackoverflow.com/a/15146073/270334
        if s.is_empty() && self.trigger == '/' {
            return Some(0);
        }

        let mut first = None;
        let mut digit = None;
        for (i, c) in s.char_indices().rev() {
            match c {
                'a'..='z' | 'A'..='Z' => first = Some(i),
                '0'..='9' => digit = Some(i),
                // scheme special
                '+' | '-' | '.' => {}
                _ => {
                    break;
                }
            }
        }

        // We don't want to extract "abc://foo" out of "1abc://foo".
        // ".abc://foo" and others are ok though, as they feel more like separators.
        if let Some(first) = first {
            if let Some(digit) = digit {
                // Comparing the byte indices with `- 1` is ok as scheme must be ASCII
                if first > 0 && first - 1 == digit {
                    return None;
                }
            }
        }
        first
    }

    fn find_end(&self, s: &str) -> Option<usize> {
        let mut round = 0;
        let mut square = 0;
        let mut curly = 0;
        let mut single_quote = false;

        let mut previous_can_be_last = true;
        let mut end = None;

        for (i, c) in s.char_indices() {
            let can_be_last = match c {
                '\u{00}'..='\u{1F}' | ' ' | '\"' | '<' | '>' | '`' | '\u{7F}'..='\u{9F}' => {
                    // These can never be part of an URL, so stop now. See RFC 3986 and RFC 3987.
                    // Some characters are not in the above list, even they are not in "unreserved"
                    // or "reserved":
                    //   '\\', '^', '{', '|', '}'
                    // The reason for this is that other link detectors also allow them. Also see
                    // below, we require the braces to be balanced.
                    break;
                }
                '?' | '!' | '.' | ',' | ':' | ';' => {
                    // These may be part of an URL but not at the end
                    false
                }
                '/' => {
                    // This may be part of an URL and at the end, but not if the previous character
                    // can't be the end of an URL
                    previous_can_be_last
                }
                '(' => {
                    round += 1;
                    false
                }
                ')' => {
                    round -= 1;
                    if round < 0 {
                        // More closing than opening brackets, stop now
                        break;
                    }
                    true
                }
                '[' => {
                    // Allowed in IPv6 address host
                    square += 1;
                    false
                }
                ']' => {
                    // Allowed in IPv6 address host
                    square -= 1;
                    if square < 0 {
                        // More closing than opening brackets, stop now
                        break;
                    }
                    true
                }
                '{' => {
                    curly += 1;
                    false
                }
                '}' => {
                    curly -= 1;
                    if curly < 0 {
                        // More closing than opening brackets, stop now
                        break;
                    }
                    true
                }
                '\'' => {
                    single_quote = !single_quote;
                    // A single quote can only be the end of an URL if there's an even number
                    !single_quote
                }
                _ => true,
            };
            if can_be_last {
                end = Some(i + c.len_utf8());
            }
            previous_can_be_last = can_be_last;
        }

        end
    }
}
