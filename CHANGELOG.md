# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-22

### Added

- Initial release.
- `specificity` — the `Specificity` `(a, b, c)` of a single complex selector
  (returns the maximum across a comma-separated list).
- `specificity_list` — one `Specificity` per comma-separated selector.
- `Specificity` — `a`/`b`/`c` components with `Ord` (a, then b, then c) and a
  `"a,b,c"` `Display`.
- Spec-accurate handling of `:is`/`:not`/`:has`/`:where`, `:nth-child(… of S)`,
  pseudo-elements, namespaces, and CSS escapes. Zero dependencies; `#![no_std]`.

[0.1.0]: https://github.com/trananhtung/specificity/releases/tag/v0.1.0
