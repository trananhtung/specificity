# specificity

[![Crates.io](https://img.shields.io/crates/v/specificity.svg)](https://crates.io/crates/specificity)
[![Documentation](https://docs.rs/specificity/badge.svg)](https://docs.rs/specificity)
[![CI](https://github.com/trananhtung/specificity/actions/workflows/ci.yml/badge.svg)](https://github.com/trananhtung/specificity/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/specificity.svg)](#license)

**Calculate the CSS specificity `(a, b, c)` of a selector** — `a` for ID selectors,
`b` for classes/attributes/pseudo-classes, `c` for type selectors/pseudo-elements.
Modern selectors are handled per the
[spec](https://www.w3.org/TR/selectors-4/#specificity-rules). The Rust counterpart of
the [`specificity`](https://www.npmjs.com/package/specificity) npm package. Zero
dependencies and `#![no_std]`.

```rust
use specificity::{specificity, Specificity};

assert_eq!(specificity("#id .cls a"), Specificity::new(1, 1, 1));
assert_eq!(specificity(":is(.a, #b)"), Specificity::new(1, 0, 0)); // max of the args
assert_eq!(specificity(":where(.a)"), Specificity::new(0, 0, 0));  // contributes nothing

// Specificity is `Ord`, so selectors compare directly.
assert!(specificity("#id") > specificity(".a.b.c"));
```

## Why specificity?

CSS linters, cascade-analysis tools, devtools, and CSS-in-JS engines all need to
know how strongly a selector matches. The rules are small but full of corners
(`:is`/`:not`/`:has` take the most specific argument, `:where` is free, `::part`
ignores its argument, namespaces and escapes don't fool the counter). Rust had this
only buried inside full CSS engines; this is the focused, dependency-free piece.

```toml
[dependencies]
specificity = "0.1"
```

## API

| Item | Purpose |
| --- | --- |
| `specificity(selector)` | The `Specificity` of a single complex selector (max of a list) |
| `specificity_list(list)` | One `Specificity` per comma-separated selector |
| `Specificity { a, b, c }` | The components; implements `Ord` (a, then b, then c) and `Display` (`"a,b,c"`) |

## Behavior

- `a` = ID selectors; `b` = classes, attribute selectors, pseudo-classes;
  `c` = type selectors and pseudo-elements. The universal `*` and combinators add
  nothing.
- `:is()`, `:not()`, `:has()` (and `:matches()`/`:-*-any()`) contribute the
  specificity of their **most specific** argument; `:where()` contributes nothing.
- `:nth-child()`/`:nth-last-child()` are pseudo-classes; an `… of S` argument adds
  the most specific selector in `S`.
- Pseudo-elements (`::before`, `::part(x)`, the legacy single-colon `:before`, …)
  count as `c`, and their functional arguments are ignored.
- Namespace prefixes (`svg|a`, `*|div`) and CSS escapes (`.foo\.bar`) are parsed
  correctly, and CSS `/* … */` comments are ignored.

## Differences from the npm package

Where keeganstreet `specificity` diverges from the CSS spec, this crate follows the
spec: `:matches()` / `:-*-any()` are treated as aliases of `:is()` (most specific
argument, not a plain pseudo-class), and a namespaced universal such as `*|*`
contributes nothing. CSS Modules' `:global()` / `:local()` are supported, taking
their argument's specificity.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.
