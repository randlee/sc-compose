mod support;

use support::{TempFixture, try_sc_lint_just_root_in, write_file};

#[test]
fn sc_lint_utility_discovery_is_scoped_to_explicit_or_checkout_local_sources() {
    let sandbox = TempFixture::new("sc-lint-discovery");
    let checkout_root = sandbox.path.join("workspace/sc-compose");
    let ancestor_sc_lint = sandbox.path.join("workspace/sc-lint/.just");
    let local_just = checkout_root.join(".just");
    let explicit_source = sandbox.path.join("explicit-sc-lint");
    let required_files = ["lint_common.py"];

    write_file(
        &ancestor_sc_lint.join("lint_common.py"),
        "unrelated ancestor",
    );
    assert_eq!(
        try_sc_lint_just_root_in(&checkout_root, None, &required_files),
        None,
        "an ancestor sc-lint checkout must not affect fixture discovery"
    );

    write_file(&local_just.join("lint_common.py"), "checkout local");
    assert_eq!(
        try_sc_lint_just_root_in(&checkout_root, None, &required_files),
        Some(local_just),
        "checkout-local utilities remain supported"
    );

    write_file(
        &explicit_source.join(".just/lint_common.py"),
        "explicit source",
    );
    assert_eq!(
        try_sc_lint_just_root_in(&checkout_root, Some(&explicit_source), &required_files),
        Some(explicit_source.join(".just")),
        "SC_LINT_SOURCE_ROOT-equivalent sources take precedence"
    );
}
