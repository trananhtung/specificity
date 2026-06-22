//! End-to-end behavioral spec for `specificity`.
//!
//! Expected values are cross-checked against the `specificity` npm package.

use specificity::{specificity, specificity_list, Specificity};

fn s(a: u32, b: u32, c: u32) -> Specificity {
    Specificity::new(a, b, c)
}

#[test]
fn basics() {
    assert_eq!(specificity("#id .cls a"), s(1, 1, 1));
    assert_eq!(specificity("*"), s(0, 0, 0));
    assert_eq!(specificity("div p"), s(0, 0, 2));
    assert_eq!(specificity("a:hover"), s(0, 1, 1));
    assert_eq!(specificity("#a#b"), s(2, 0, 0));
    assert_eq!(specificity(".x[y].z"), s(0, 3, 0));
    assert_eq!(specificity("a.b#c[d]:hover::before"), s(1, 3, 2));
    assert_eq!(specificity("input:checked"), s(0, 1, 1));
}

#[test]
fn pseudo_elements() {
    assert_eq!(specificity("::before"), s(0, 0, 1));
    assert_eq!(specificity(":before"), s(0, 0, 1)); // legacy single-colon
    assert_eq!(specificity("::after"), s(0, 0, 1));
    assert_eq!(specificity("::first-line"), s(0, 0, 1));
    assert_eq!(specificity("::part(x)"), s(0, 0, 1)); // functional pseudo-element, args ignored
    assert_eq!(specificity("::slotted(.a)"), s(0, 0, 1));
    assert_eq!(specificity("div::before:hover"), s(0, 1, 2));
}

#[test]
fn functional_pseudo_classes() {
    assert_eq!(specificity(":is(.a, #b)"), s(1, 0, 0)); // max of args
    assert_eq!(specificity(":not(.a, #b)"), s(1, 0, 0));
    assert_eq!(specificity(":has(.a, #b)"), s(1, 0, 0));
    assert_eq!(specificity(":where(.a)"), s(0, 0, 0)); // contributes nothing
    assert_eq!(specificity(":not(:is(.a,#b))"), s(1, 0, 0)); // nested
    assert_eq!(specificity(":host(.x)"), s(0, 1, 0)); // host: pseudo-class, arg ignored
}

#[test]
fn nth_child() {
    assert_eq!(specificity("li:nth-child(2n+1)"), s(0, 1, 1));
    assert_eq!(specificity(":nth-child(odd)"), s(0, 1, 0));
    // `of S` adds the max specificity of S
    assert_eq!(specificity("li:nth-child(2n of .a, #id)"), s(1, 1, 1));
}

#[test]
fn attributes_and_namespaces() {
    assert_eq!(specificity("[href^=\"http\"]"), s(0, 1, 0));
    assert_eq!(specificity("[data-x=y i]"), s(0, 1, 0));
    assert_eq!(specificity("svg|a"), s(0, 0, 1)); // namespace prefix ignored
    assert_eq!(specificity("*|div"), s(0, 0, 1));
    assert_eq!(specificity("|div"), s(0, 0, 1));
    assert_eq!(specificity("*|*"), s(0, 0, 0));
}

#[test]
fn combinators_are_ignored() {
    assert_eq!(specificity("a > b + c ~ d"), s(0, 0, 4));
    assert_eq!(specificity("* + *"), s(0, 0, 0));
    assert_eq!(specificity("a || b"), s(0, 0, 2)); // column combinator
}

#[test]
fn escapes() {
    assert_eq!(specificity(".foo\\.bar"), s(0, 1, 0)); // one class "foo.bar"
    assert_eq!(specificity("#\\31 23"), s(1, 0, 0)); // escaped id
}

#[test]
fn lists() {
    // single function returns the max across a list
    assert_eq!(specificity(".a, #b"), s(1, 0, 0));
    // list function returns one per selector
    assert_eq!(specificity_list(".a, #b"), [s(0, 1, 0), s(1, 0, 0)]);
    assert_eq!(
        specificity_list("#a, .b, c"),
        [s(1, 0, 0), s(0, 1, 0), s(0, 0, 1)]
    );
}

// ---------------------------------------------------------------------------
// Regression: adversarial-review findings
// ---------------------------------------------------------------------------

#[test]
fn comments_are_stripped() {
    assert_eq!(specificity(".a/* #id */"), s(0, 1, 0));
    assert_eq!(specificity("/* x */.a.b"), s(0, 2, 0));
    assert_eq!(specificity("div/* c */.foo"), s(0, 1, 1));
    // a comment containing a comma must not split the list
    assert_eq!(specificity_list(".a /* , */ .b"), [s(0, 2, 0)]);
    // a comment-like sequence inside an attribute string is preserved
    assert_eq!(specificity("[a=\"/*\"]"), s(0, 1, 0));
}

#[test]
fn css_modules_global_local() {
    // :global()/:local() take their argument's specificity (per keeganstreet)
    assert_eq!(specificity(":global(#a .b .c)"), s(1, 2, 0));
    assert_eq!(specificity(":local(#a)"), s(1, 0, 0));
    assert_eq!(specificity(":global(div)"), s(0, 0, 1));
    // bare :global/:local contribute nothing
    assert_eq!(specificity(":global"), s(0, 0, 0));
    assert_eq!(specificity(":local"), s(0, 0, 0));
}

#[test]
fn matches_and_any_are_is_aliases() {
    // per Selectors-4, :matches()/:-*-any() are aliases of :is() (most specific arg)
    assert_eq!(specificity(":matches(#a)"), s(1, 0, 0));
    assert_eq!(specificity(":-webkit-any(.a, #b)"), s(1, 0, 0));
}

#[test]
fn deep_nesting_does_not_overflow() {
    // pathologically nested input degrades gracefully instead of aborting the process
    let deep = format!("{}{}{}", ":is(".repeat(5000), ".a", &")".repeat(5000));
    let _ = specificity(&deep); // must return, not stack-overflow
                                // reasonable nesting still counts correctly
    let ten = format!("{}{}{}", ":is(".repeat(10), "#x", &")".repeat(10));
    assert_eq!(specificity(&ten), s(1, 0, 0));
}

#[test]
fn ordering_and_display() {
    assert!(specificity("#id") > specificity(".a.b.c"));
    assert!(specificity(".a") > specificity("div span"));
    assert_eq!(specificity("#a.b c").to_string(), "1,1,1");
    assert_eq!(Specificity::default(), s(0, 0, 0));
}
