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

pub fn game(stage: usize, character: i16) -> crate::game::Game {
    use std::{
        cell::RefCell,
        io::{Read, Seek, SeekFrom},
        path::PathBuf,
    };
    use tenchu_assets::{
        ReadAt, cue, disc::Disc, executable::Executable, game::GameAssets, stage::MissionAssets,
    };
    struct File(RefCell<fs::File>, u64);
    impl ReadAt for File {
        fn byte_len(&self) -> u64 {
            self.1
        }
        fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> crate::Result<()> {
            let mut file = self.0.borrow_mut();
            file.seek(SeekFrom::Start(offset))
                .map_err(|e| e.to_string())?;
            file.read_exact(output).map_err(|e| e.to_string())
        }
    }
    let path = PathBuf::from(std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC"));
    let track = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("cue"))
        .then(|| cue::data_track(&fs::read_to_string(&path).unwrap()).unwrap());
    let binary = track
        .as_ref()
        .map_or_else(|| path.clone(), |t| path.parent().unwrap().join(&t.file));
    let file = fs::File::open(binary).unwrap();
    let length = file.metadata().unwrap().len();
    let file = File(RefCell::new(file), length);
    let disc = match track {
        Some(t) => Disc::open(file, t.format, t.start_sector),
        None => Disc::detect(file),
    }
    .unwrap();
    let assets = GameAssets::open(disc).unwrap();
    let mission = MissionAssets::load(&assets, stage, character, 1).unwrap();
    crate::game::Game::new(
        mission,
        &Executable::new(&assets.executable).unwrap(),
        character,
        1,
    )
    .unwrap()
}
