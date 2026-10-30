use std::{env, path::PathBuf, process::Command};

fn output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    for name in ["TENCHU_BUILD_REVISION", "TARGET", "PROFILE", "RUSTC"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    for path in [
        "src",
        "web",
        "../tenchu-core/src",
        "../tenchu-assets/src",
        "../tenchu-audio/src",
        "../../Cargo.lock",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    if let Some(git) = output("git", &["rev-parse", "--absolute-git-dir"]) {
        let git = PathBuf::from(git);
        for path in [git.join("HEAD"), git.join("index"), git.join("packed-refs")] {
            if path.exists() {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
        if let Some(reference) = output("git", &["symbolic-ref", "-q", "HEAD"]) {
            println!("cargo:rerun-if-changed={}", git.join(reference).display());
        }
    }
    let revision = env::var("TENCHU_BUILD_REVISION")
        .ok()
        .or_else(|| {
            let mut revision = output("git", &["rev-parse", "HEAD"])?;
            if output("git", &["status", "--porcelain", "--untracked-files=no"])
                .is_some_and(|s| !s.is_empty())
            {
                revision.push_str("-dirty");
            }
            Some(revision)
        })
        .unwrap_or_else(|| "unknown".into());
    let rustc = output(
        &env::var("RUSTC").unwrap_or_else(|_| "rustc".into()),
        &["--version"],
    )
    .unwrap_or_else(|| "unknown".into());
    for (key, value) in [
        ("TENCHU_BUILD_REVISION", revision),
        ("TENCHU_BUILD_TARGET", env::var("TARGET").unwrap()),
        ("TENCHU_BUILD_PROFILE", env::var("PROFILE").unwrap()),
        ("TENCHU_BUILD_COMPILER", rustc),
    ] {
        println!("cargo:rustc-env={key}={value}");
    }
}
