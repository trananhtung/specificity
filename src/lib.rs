//! # specificity — calculate CSS selector specificity
//!
//! Compute the [specificity](https://www.w3.org/TR/selectors-4/#specificity-rules)
//! `(a, b, c)` of a CSS selector — `a` counts ID selectors, `b` counts classes,
//! attributes and pseudo-classes, and `c` counts type selectors and pseudo-elements.
//! Modern selectors are handled per the spec: `:is()`, `:not()`, `:has()` and
//! `:nth-child(… of S)` take the specificity of their most specific argument, and
//! `:where()` contributes nothing. The Rust counterpart of the
//! [`specificity`](https://www.npmjs.com/package/specificity) npm package. Zero
//! dependencies and `#![no_std]`.
//!
//! ```
//! use specificity::{specificity, Specificity};
//!
//! assert_eq!(specificity("#id .cls a"), Specificity::new(1, 1, 1));
//! assert_eq!(specificity(":is(.a, #b)"), Specificity::new(1, 0, 0)); // max of args
//! assert_eq!(specificity(":where(.a)"), Specificity::new(0, 0, 0));  // contributes 0
//!
//! // Specificity is Ord, so you can compare selectors directly.
//! assert!(specificity("#id") > specificity(".a.b.c"));
//! ```
//!
//! [`specificity`] takes a single complex selector (returning the maximum if you pass
//! a comma-separated list); use [`specificity_list`] to get one value per selector.

#![no_std]
#![doc(html_root_url = "https://docs.rs/specificity/0.1.0")]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

// Compile-test the README's examples as part of `cargo test`.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// A CSS specificity value: `a` (IDs), `b` (classes/attributes/pseudo-classes), and
/// `c` (type selectors/pseudo-elements). Ordered `a`, then `b`, then `c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity {
    /// The number of ID selectors.
    pub a: u32,
    /// The number of class, attribute, and pseudo-class selectors.
    pub b: u32,
    /// The number of type selectors and pseudo-elements.
    pub c: u32,
}

impl Specificity {
    /// Construct a specificity from its three components.
    #[must_use]
    pub const fn new(a: u32, b: u32, c: u32) -> Self {
        Self { a, b, c }
    }

    fn add(self, other: Specificity) -> Self {
        Self {
            a: self.a + other.a,
            b: self.b + other.b,
            c: self.c + other.c,
        }
    }
}

impl fmt::Display for Specificity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.a, self.b, self.c)
    }
}

/// Calculate the specificity of a single complex `selector`.
///
/// If `selector` is a comma-separated list, the maximum specificity in the list is
/// returned; use [`specificity_list`] for the per-selector values.
///
/// ```
/// assert_eq!(specificity::specificity("a.b#c").to_string(), "1,1,1");
/// ```
#[must_use]
pub fn specificity(selector: &str) -> Specificity {
    let chars = strip_comments(&selector.chars().collect::<Vec<_>>());
    max_of_list(&chars, 0)
}

/// Calculate the specificity of each selector in a comma-separated `list`.
///
/// ```
/// use specificity::{specificity_list, Specificity};
/// assert_eq!(specificity_list(".a, #b"), [Specificity::new(0, 1, 0), Specificity::new(1, 0, 0)]);
/// ```
#[must_use]
pub fn specificity_list(list: &str) -> Vec<Specificity> {
    let chars = strip_comments(&list.chars().collect::<Vec<_>>());
    split_top_level_commas(&chars)
        .into_iter()
        .map(|sel| count_selector(&sel, 0))
        .collect()
}

/// A bound on functional-pseudo nesting depth, so absurdly nested input (e.g.
/// thousands of `:is(`) degrades gracefully instead of overflowing the stack. Real
/// selectors nest only a handful of levels.
const MAX_DEPTH: u32 = 1024;

/// The maximum specificity across a (possibly comma-separated) selector list.
fn max_of_list(chars: &[char], depth: u32) -> Specificity {
    split_top_level_commas(chars)
        .into_iter()
        .map(|sel| count_selector(&sel, depth))
        .max()
        .unwrap_or_default()
}

/// Count the specificity of one complex selector (no top-level commas).
fn count_selector(chars: &[char], depth: u32) -> Specificity {
    let mut spec = Specificity::default();
    if depth > MAX_DEPTH {
        return spec; // refuse to recurse further into pathologically nested input
    }
    let len = chars.len();
    let mut i = 0;
    while i < len {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' | '\x0c' | '>' | '+' | '~' => i += 1,
            '|' if chars.get(i + 1) == Some(&'|') => i += 2, // column combinator
            '#' => {
                i = read_ident(chars, i + 1);
                spec.a += 1;
            }
            '.' => {
                i = read_ident(chars, i + 1);
                spec.b += 1;
            }
            '[' => {
                i = skip_attribute(chars, i);
                spec.b += 1;
            }
            ':' => i = read_pseudo(chars, i, &mut spec, depth),
            '*' | '|' => {
                let (next, c) = read_type(chars, i);
                i = next;
                spec.c += c;
            }
            ch if is_ident_start(ch) => {
                let (next, c) = read_type(chars, i);
                i = next;
                spec.c += c;
            }
            _ => i += 1, // stray punctuation / unsupported — ignore
        }
    }
    spec
}

/// Handle a `:` … run (pseudo-class or pseudo-element) starting at `chars[i] == ':'`,
/// updating `spec` and returning the index just past it.
fn read_pseudo(chars: &[char], i: usize, spec: &mut Specificity, depth: u32) -> usize {
    // Pseudo-element: `::name` (args, if any, do not add to specificity).
    if chars.get(i + 1) == Some(&':') {
        let end = read_ident(chars, i + 2);
        spec.c += 1;
        return skip_optional_parens(chars, end);
    }

    let name_end = read_ident(chars, i + 1);
    let name = lower(&chars[i + 1..name_end]);
    let functional = chars.get(name_end) == Some(&'(');

    if !functional {
        match name.as_str() {
            // The four legacy pseudo-elements may be written with a single colon.
            "before" | "after" | "first-line" | "first-letter" => spec.c += 1,
            // CSS Modules markers contribute nothing on their own.
            "global" | "local" => {}
            _ => spec.b += 1, // ordinary pseudo-class
        }
        return name_end;
    }

    let (inner, end) = read_parens(chars, name_end);
    match name.as_str() {
        "where" => {} // contributes nothing
        // Take the most specific argument. `:matches`/`:-*-any` are the legacy
        // aliases of `:is`; `:global`/`:local` are CSS Modules.
        "is" | "not" | "has" | "matches" | "-moz-any" | "-webkit-any" | "global" | "local" => {
            *spec = spec.add(max_of_list(&inner, depth + 1));
        }
        "nth-child" | "nth-last-child" => {
            spec.b += 1;
            if let Some(of) = selector_after_of(&inner) {
                *spec = spec.add(max_of_list(of, depth + 1));
            }
        }
        _ => spec.b += 1, // any other functional pseudo-class; arguments ignored
    }
    end
}

/// Remove CSS `/* … */` comments, leaving any inside a quoted string intact.
fn strip_comments(chars: &[char]) -> Vec<char> {
    let len = chars.len();
    let mut out = Vec::with_capacity(len);
    let mut i = 0;
    let mut quote: Option<char> = None;
    while i < len {
        let c = chars[i];
        if let Some(q) = quote {
            out.push(c);
            i += 1;
            if c == '\\' {
                if i < len {
                    out.push(chars[i]);
                    i += 1;
                }
            } else if c == q {
                quote = None;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i < len && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                i += 1;
            }
            i = (i + 2).min(len);
        } else {
            if c == '"' || c == '\'' {
                quote = Some(c);
            }
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Read a type/universal selector (handling a namespace prefix) starting at `i`.
/// Returns the index just past it and how much it adds to `c` (1 for a type, 0 for `*`).
fn read_type(chars: &[char], i: usize) -> (usize, u32) {
    // Empty namespace prefix: `|element`.
    if chars[i] == '|' {
        return read_element(chars, i + 1);
    }
    let after_prefix = if chars[i] == '*' {
        i + 1
    } else {
        read_ident(chars, i)
    };
    // A `|` (that is not `||` or `|=`) makes what we read a namespace prefix.
    if chars.get(after_prefix) == Some(&'|')
        && chars.get(after_prefix + 1) != Some(&'|')
        && chars.get(after_prefix + 1) != Some(&'=')
    {
        return read_element(chars, after_prefix + 1);
    }
    let c = u32::from(chars[i] != '*');
    (after_prefix, c)
}

/// Read the element after a namespace `|`: `*` adds 0, a type name adds 1.
fn read_element(chars: &[char], i: usize) -> (usize, u32) {
    if chars.get(i) == Some(&'*') {
        return (i + 1, 0);
    }
    let end = read_ident(chars, i);
    (end, u32::from(end > i))
}

// ---------------------------------------------------------------------------
// Low-level scanning helpers
// ---------------------------------------------------------------------------

/// Whether `ch` can start a CSS identifier (for a type selector).
fn is_ident_start(ch: char) -> bool {
    ch == '-' || ch == '_' || ch == '\\' || ch.is_alphabetic() || (ch as u32) >= 0x80
}

/// Consume a CSS identifier (including escapes), returning the index past it.
fn read_ident(chars: &[char], mut i: usize) -> usize {
    let len = chars.len();
    while i < len {
        let ch = chars[i];
        if ch == '\\' {
            i += 1;
            if i >= len {
                break;
            }
            if chars[i].is_ascii_hexdigit() {
                let mut k = 0;
                while i < len && k < 6 && chars[i].is_ascii_hexdigit() {
                    i += 1;
                    k += 1;
                }
                if i < len && chars[i].is_whitespace() {
                    i += 1;
                }
            } else {
                i += 1; // escaped literal character
            }
        } else if ch == '-' || ch == '_' || ch.is_alphanumeric() || (ch as u32) >= 0x80 {
            i += 1;
        } else {
            break;
        }
    }
    i
}

/// Skip an attribute selector `[ … ]` (respecting quoted values), starting at `[`.
fn skip_attribute(chars: &[char], mut i: usize) -> usize {
    let len = chars.len();
    i += 1; // past '['
    while i < len {
        match chars[i] {
            ']' => return i + 1,
            q @ ('"' | '\'') => i = skip_string(chars, i + 1, q),
            _ => i += 1,
        }
    }
    i
}

/// Skip a quoted string body starting just after the opening quote `q`.
fn skip_string(chars: &[char], mut i: usize, q: char) -> usize {
    let len = chars.len();
    while i < len {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i] == q {
            return i + 1;
        }
        i += 1;
    }
    i
}

/// If `chars[i]` is `(`, skip the balanced parenthesised group; otherwise return `i`.
fn skip_optional_parens(chars: &[char], i: usize) -> usize {
    if chars.get(i) == Some(&'(') {
        read_parens(chars, i).1
    } else {
        i
    }
}

/// Read a balanced parenthesised group starting at `chars[i] == '('`. Returns the
/// inner characters and the index just past the closing `)`.
fn read_parens(chars: &[char], i: usize) -> (Vec<char>, usize) {
    let len = chars.len();
    let start = i + 1;
    let mut j = start;
    let mut depth = 1;
    while j < len {
        match chars[j] {
            q @ ('"' | '\'') => {
                j = skip_string(chars, j + 1, q);
                continue;
            }
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return (chars[start..j].to_vec(), j + 1);
                }
            }
            _ => {}
        }
        j += 1;
    }
    (chars[start..].to_vec(), j)
}

/// Split a selector list on top-level commas (ignoring commas inside `()`, `[]`,
/// and strings).
fn split_top_level_commas(chars: &[char]) -> Vec<Vec<char>> {
    let len = chars.len();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let mut depth = 0i32;
    while i < len {
        match chars[i] {
            q @ ('"' | '\'') => {
                i = skip_string(chars, i + 1, q);
                continue;
            }
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ',' if depth <= 0 => {
                parts.push(chars[start..i].to_vec());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(chars[start..].to_vec());
    parts
}

/// The selector list following an ` of ` keyword inside an `nth-*` argument, if any.
fn selector_after_of(inner: &[char]) -> Option<&[char]> {
    let len = inner.len();
    let mut i = 0;
    while i + 3 < len {
        if inner[i].is_whitespace()
            && (inner[i + 1] == 'o' || inner[i + 1] == 'O')
            && (inner[i + 2] == 'f' || inner[i + 2] == 'F')
            && inner[i + 3].is_whitespace()
        {
            return Some(&inner[i + 4..]);
        }
        i += 1;
    }
    None
}

/// Lowercase a slice of chars into an owned `String`.
fn lower(chars: &[char]) -> String {
    chars.iter().flat_map(|c| c.to_lowercase()).collect()
}
