#[path = "../build/build_support.rs"]
mod build_support;

#[test]
fn formatter_distinguishes_clean_dirty_and_archive_builds() {
    assert_eq!(
        build_support::format_build_version("0.14.0", "deadbeef", false),
        "0.14.0+gdeadbeef"
    );
    assert_eq!(
        build_support::format_build_version("0.14.0", "deadbeef", true),
        "0.14.0+gdeadbeef.dirty"
    );
    assert_eq!(
        build_support::format_build_version("0.14.0", "unknown", true),
        "0.14.0"
    );
}

#[test]
fn compiled_identity_has_checkout_metadata_when_git_is_available() {
    assert!(env!("LOCTREE_BUILD_VERSION").starts_with(env!("CARGO_PKG_VERSION")));
    if env!("LOCTREE_GIT_COMMIT") != "unknown" {
        assert!(
            env!("LOCTREE_BUILD_VERSION").contains(&format!("+g{}", env!("LOCTREE_GIT_COMMIT")))
        );
    }
}
