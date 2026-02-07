//! Domain name related scanning, used by both email and URL scanners.
//!
//! This is called domains for familiarity but it's about the authority part of URLs as defined in
//! https://datatracker.ietf.org/doc/html/rfc3986#section-3.2
//!
//! ```text
//! authority   = [ userinfo "@" ] host [ ":" port ]
//!
//!
//! userinfo    = *( unreserved / pct-encoded / sub-delims / ":" )
//!
//! host        = IP-literal / IPv4address / reg-name
//!
//! IP-literal = "[" ( IPv6address / IPvFuture  ) "]"
//!
//! IPv4address = dec-octet "." dec-octet "." dec-octet "." dec-octet
//!
//! reg-name    = *( unreserved / pct-encoded / sub-delims )
//!
//!
//! unreserved  = ALPHA / DIGIT / "-" / "." / "_" / "~"
//!
//! sub-delims  = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
//!
//! pct-encoded = "%" HEXDIG HEXDIG
//! ```

use std::char;

pub(crate) fn find_authority_end(
    s: &str,
    mut userinfo_allowed: bool,
    require_host: bool,
    port_allowed: bool,
    iri_parsing_enabled: bool,
) -> (Option<usize>, Option<usize>) {
    // Handle IPv6 literals: IP-literal = "[" ( IPv6address / IPvFuture ) "]"
    // Per RFC 2732 and RFC 3986
    if s.starts_with('[') {
        return find_ipv6_authority_end(s, port_allowed);
    }

    let mut end = Some(0);

    let mut maybe_last_dot = None;
    let mut last_dot = None;
    let mut number_dots = 0;
    let mut dot_allowed = false;
    let mut hyphen_allowed = false;
    let mut all_numeric = true;
    let mut maybe_host = true;
    let mut host_ended = false;

    for (i, c) in s.char_indices() {
        let can_be_last = match c {
            // ALPHA
            'a'..='z' | 'A'..='Z' => {
                // Can start or end a domain label, but not numeric
                dot_allowed = true;
                hyphen_allowed = true;
                last_dot = maybe_last_dot;
                all_numeric = false;

                if host_ended {
                    maybe_host = false;
                }

                !require_host || !host_ended
            }
            // International characters (IRI support)
            '\u{80}'..=char::MAX => {
                if !iri_parsing_enabled {
                    break;
                }
                // Exclude Unicode whitespace (e.g., NBSP, EM SPACE, IDEOGRAPHIC SPACE)
                if c.is_whitespace() {
                    break;
                }
                // Can start or end a domain label, but not numeric
                dot_allowed = true;
                hyphen_allowed = true;
                last_dot = maybe_last_dot;
                all_numeric = false;

                if host_ended {
                    maybe_host = false;
                }

                !require_host || !host_ended
            }

            // DIGIT
            '0'..='9' => {
                // Same as above, except numeric
                dot_allowed = true;
                hyphen_allowed = true;
                if last_dot != maybe_last_dot {
                    last_dot = maybe_last_dot;
                    number_dots += 1;
                }

                if host_ended {
                    maybe_host = false;
                }

                !require_host || !host_ended
            }
            // unreserved
            '-' => {
                // Hyphen can't be at start of a label, e.g. `-b` in `a.-b.com`
                if !hyphen_allowed {
                    maybe_host = false;
                }
                // Hyphen can't be at end of a label, e.g. `b-` in `a.b-.com`
                dot_allowed = false;
                all_numeric = false;

                !require_host
            }
            '.' => {
                if !dot_allowed {
                    // Label can't be empty, e.g. `.example.com` or `a..com`
                    host_ended = true;
                }
                dot_allowed = false;
                hyphen_allowed = false;
                maybe_last_dot = Some(i);

                false
            }
            '_' | '~' => {
                // Hostnames can't contain these and we don't want to treat them as delimiters.
                maybe_host = false;

                false
            }
            // sub-delims
            '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' => {
                // Can't be in hostnames, but we treat them as delimiters
                host_ended = true;

                if !userinfo_allowed && require_host {
                    // We don't have to look further
                    break;
                }

                false
            }
            ':' => {
                // Could be in userinfo, or we're getting a port now.
                if !userinfo_allowed && !port_allowed {
                    break;
                }

                // Don't advance the last dot when we get to port numbers
                maybe_last_dot = last_dot;

                false
            }
            '@' => {
                if !userinfo_allowed {
                    // We already had userinfo, can't have another `@` in a valid authority.
                    return (None, None);
                }

                // Sike! Everything before this has been userinfo, so let's reset our
                // opinions about all the host bits.
                userinfo_allowed = false;

                maybe_last_dot = None;
                last_dot = None;
                dot_allowed = false;
                hyphen_allowed = false;
                all_numeric = true;
                maybe_host = true;
                host_ended = false;

                false
            }
            '/' => {
                if !require_host {
                    // For schemes where we allow anything, we want to stop at delimiter characters
                    // except if we get a slash closing the URL, which happened here.
                    end = Some(i);
                }
                break;
            }
            _ => {
                // Anything else, this might be the end of the authority (can be empty).
                // Now let the rest of the code handle checking whether the end of the URL is
                // valid.
                break;
            }
        };

        if can_be_last {
            end = Some(i + c.len_utf8());
        }
    }

    if require_host {
        if maybe_host {
            if all_numeric {
                // For IPv4 addresses, require 4 numbers
                if number_dots != 3 {
                    return (None, None);
                }
            } else {
                // If we have something that is not just numeric (not an IP address),
                // check that the TLD looks reasonable. This is to avoid linking things like
                // `abc@v1.1`.
                if let Some(last_dot) = last_dot {
                    if !valid_tld(&s[last_dot + 1..]) {
                        return (None, None);
                    }
                }
            }

            (end, last_dot)
        } else {
            (None, None)
        }
    } else {
        (end, last_dot)
    }
}

fn valid_tld(tld: &str) -> bool {
    tld.chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .take(2)
        .count()
        >= 2
}

/// Handle IPv6 literal addresses in the authority component.
///
/// Parses `[IPv6address]` optionally followed by `:port`.
/// Returns (end_position, None) since IPv6 literals don't have a "last dot" for TLD checking.
fn find_ipv6_authority_end(s: &str, port_allowed: bool) -> (Option<usize>, Option<usize>) {
    debug_assert!(s.starts_with('['));

    let mut end = 1; // Skip opening '['
    let mut found_close = false;

    // Scan for valid IPv6 characters until we find ']'
    for (i, c) in s[1..].char_indices() {
        match c {
            // Valid IPv6 characters: hex digits, colons, dots (for IPv4-mapped addresses)
            '0'..='9' | 'a'..='f' | 'A'..='F' | ':' | '.' => {
                end = 1 + i + c.len_utf8();
            }
            ']' => {
                end = 1 + i + 1; // Include the ']'
                found_close = true;
                break;
            }
            _ => {
                // Invalid character in IPv6 literal
                return (None, None);
            }
        }
    }

    if !found_close {
        // No closing bracket found
        return (None, None);
    }

    // Check what comes after the ']'
    let rest = &s[end..];

    if rest.is_empty() {
        // Just the IPv6 address, no port or path
        return (Some(end), None);
    }

    match rest.chars().next() {
        Some(':') if port_allowed => {
            // Parse port number
            let port_start = end + 1;
            let mut port_end = port_start;

            for (i, c) in s[port_start..].char_indices() {
                if c.is_ascii_digit() {
                    port_end = port_start + i + 1;
                } else {
                    break;
                }
            }

            if port_end > port_start {
                // Valid port found
                (Some(port_end), None)
            } else {
                // Colon but no port digits - still valid, authority ends at ']'
                (Some(end), None)
            }
        }
        Some('/') | Some('?') | Some('#') | None => {
            // Path, query, fragment, or end of string - authority ends here
            (Some(end), None)
        }
        Some(':') => {
            // Port not allowed but found colon
            (Some(end), None)
        }
        _ => {
            // Invalid character after ']'
            (Some(end), None)
        }
    }
}
