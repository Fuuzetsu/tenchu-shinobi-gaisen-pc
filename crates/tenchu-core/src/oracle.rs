use std::{
    fs,
    process::{Command, Stdio},
};

pub fn source(file: &str) -> String {
    let directory =
        std::path::PathBuf::from(std::env::var_os("TENCHU_DECOMP").expect("Set TENCHU_DECOMP"));
    fs::read_to_string(directory.join("src/main.exe").join(file)).unwrap()
}

pub fn function<'a>(source: &'a str, signature: &str) -> &'a str {
    let start = source.find(&format!("\n{signature}")).unwrap() + 1;
    let mut depth = 0;
    for (i, c) in source[start..].char_indices() {
        if c == '{' {
            depth += 1;
        }
        if c == '}' {
            depth -= 1;
            if depth == 0 {
                return &source[start..start + i + 1];
            }
        }
    }
    panic!("Unterminated C function {signature}");
}

pub fn run(harness: &str, input: &str) -> Vec<Vec<i32>> {
    let directory = std::env::temp_dir().join(format!(
        "tenchu-oracle-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("oracle.c"), harness).unwrap();
    fs::write(directory.join("input"), input).unwrap();
    let build = Command::new("cc")
        .args(["-std=c99", "-O2", "-fwrapv", "oracle.c", "-o", "oracle"])
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = Command::new(directory.join("oracle"))
        .stdin(Stdio::from(
            fs::File::open(directory.join("input")).unwrap(),
        ))
        .output()
        .unwrap();
    assert!(output.status.success());
    let result = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(|v| v.parse().unwrap())
                .collect()
        })
        .collect();
    fs::remove_dir_all(directory).unwrap();
    result
}
