#[path = "build/build_support.rs"]
mod build_support;

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build/build_support.rs");
    for name in [
        "LOCTREE_GIT_COMMIT",
        "LOCTREE_GIT_DIRTY",
        "LOCTREE_GIT_DESCRIBE",
        "LOCTREE_BUILD_VERSION",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        println!("cargo:rerun-if-changed={git_dir}/index");
    }
    if let Some(common_dir) = git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]) {
        println!("cargo:rerun-if-changed={common_dir}/packed-refs");
        if let Some(head_ref) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!("cargo:rerun-if-changed={common_dir}/{head_ref}");
        }
    }

    let commit = env_value("LOCTREE_GIT_COMMIT")
        .or_else(|| git(&["rev-parse", "--short=8", "HEAD"]))
        .unwrap_or_else(|| "unknown".to_owned());
    let dirty = env_value("LOCTREE_GIT_DIRTY")
        .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "dirty"))
        .unwrap_or_else(|| {
            git(&["status", "--porcelain"])
                .map(|status| !status.is_empty())
                .unwrap_or(false)
        });
    let describe = env_value("LOCTREE_GIT_DESCRIBE")
        .or_else(|| git(&["describe", "--always", "--dirty", "--tags"]))
        .unwrap_or_else(|| commit.clone());
    let package_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_owned());
    let build_version = env_value("LOCTREE_BUILD_VERSION")
        .unwrap_or_else(|| build_support::format_build_version(&package_version, &commit, dirty));

    println!("cargo:rustc-env=LOCTREE_GIT_COMMIT={commit}");
    println!(
        "cargo:rustc-env=LOCTREE_GIT_DIRTY={}",
        if dirty { "1" } else { "0" }
    );
    println!("cargo:rustc-env=LOCTREE_GIT_DESCRIBE={describe}");
    println!("cargo:rustc-env=LOCTREE_BUILD_VERSION={build_version}");
}
