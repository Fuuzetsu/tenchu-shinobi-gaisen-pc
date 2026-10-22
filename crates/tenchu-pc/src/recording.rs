use crate::session::Options;
#[cfg(not(target_arch = "wasm32"))]
use crate::source::DiscSource;
#[cfg(not(target_arch = "wasm32"))]
use std::ffi::OsString;
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(not(target_arch = "wasm32"))]
use tenchu_assets::game::GameAssets;
use tenchu_assets::{Result, script::Language};
#[cfg(not(target_arch = "wasm32"))]
use tenchu_core::game::Game;

const HEADER: &str = "# tenchu-recording 1";

#[derive(Clone, Copy, Debug)]
pub struct Start {
    pub options: Options,
    pub seed: u32,
    pub inventory: [u8; 20],
}

impl Start {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(self, assets: &GameAssets<DiscSource>) -> Result<Game> {
        self.options
            .load_equipped(assets, self.seed, self.inventory)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn parse(input: &str) -> Result<Option<Self>> {
        let mut lines = input.lines().filter(|line| !line.trim().is_empty());
        let Some(first) = lines.next().map(str::trim) else {
            return Ok(None);
        };
        if !first.starts_with("# tenchu-recording") {
            return Ok(None);
        }
        if first != HEADER {
            return Err(format!("Unsupported recording header: {first}"));
        }
        let mut field = |prefix: &str| -> Result<&str> {
            lines
                .next()
                .and_then(|line| line.trim().strip_prefix(prefix))
                .ok_or_else(|| format!("Recording needs {prefix}"))
        };
        let args: Vec<OsString> = field("# options ")?
            .split_whitespace()
            .map(Into::into)
            .collect();
        let keys = [
            "--character",
            "--mission",
            "--layout",
            "--difficulty",
            "--language",
        ];
        if args.len() != keys.len() * 2
            || keys
                .iter()
                .any(|key| args.chunks_exact(2).filter(|pair| pair[0] == *key).count() != 1)
        {
            return Err("Recording needs all five launch options exactly once".into());
        }
        let options = Options::parse(&args)?;
        let seed = field("# seed ")?
            .parse()
            .map_err(|_| "Invalid recording seed")?;
        let inventory = field("# inventory ")?
            .split_whitespace()
            .map(|value| {
                value
                    .parse::<u8>()
                    .map_err(|_| "Invalid recorded inventory")
            })
            .collect::<std::result::Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| "Recording needs twenty inventory counts")?;
        Ok(Some(Self {
            options,
            seed,
            inventory,
        }))
    }

    fn write(self, output: &mut impl Write) -> std::io::Result<()> {
        let options = self.options;
        let character = if options.character == 0 {
            "rikimaru"
        } else {
            "ayame"
        };
        let mission = if options.mission == 0 {
            "training".to_owned()
        } else {
            options.mission.to_string()
        };
        let layout = if options.layout == 0 {
            "random".to_owned()
        } else {
            options.layout.to_string()
        };
        let difficulty = options.difficulty.name();
        let language = match options.language.unwrap_or_default() {
            Language::English => "english",
            Language::French => "french",
            Language::Italian => "italian",
            Language::Japanese => "japanese",
        };
        writeln!(output, "{HEADER}")?;
        writeln!(
            output,
            "# options --character {character} --mission {mission} --layout {layout} --difficulty {difficulty} --language {language}"
        )?;
        writeln!(output, "# seed {}", self.seed)?;
        write!(output, "# inventory")?;
        for count in self.inventory {
            write!(output, " {count}")?;
        }
        writeln!(output)
    }
}

pub struct Recorder<W: Write> {
    output: W,
    pending: Option<(u32, u16)>,
    unflushed: u32,
}

impl<W: Write> Recorder<W> {
    pub fn new(mut output: W, start: Start) -> Result<Self> {
        start.write(&mut output).map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        Ok(Self {
            output,
            pending: None,
            unflushed: 0,
        })
    }

    fn write_run(&mut self) -> Result<()> {
        if let Some((count, buttons)) = self.pending.take() {
            writeln!(self.output, "{count} {buttons:04x}").map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn push(&mut self, buttons: u16) -> Result<()> {
        if let Some((count, _)) = self.pending.as_mut().filter(|(_, last)| *last == buttons) {
            *count += 1;
        } else {
            self.write_run()?;
            self.pending = Some((1, buttons));
        }
        self.unflushed += 1;
        if self.unflushed == 30 {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        self.write_run()?;
        self.output.flush().map_err(|e| e.to_string())?;
        self.unflushed = 0;
        Ok(())
    }
}

impl Recorder<BufWriter<File>> {
    pub fn create(directory: &Path, start: Start) -> Result<Self> {
        std::fs::create_dir_all(directory).map_err(|e| format!("{}: {e}", directory.display()))?;
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let path = directory.join(format!(
            "mission{}-{time}-{}.inputs",
            start.options.mission,
            std::process::id()
        ));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let recorder = Self::new(BufWriter::new(file), start)?;
        eprintln!("Recording: {}", path.display());
        Ok(recorder)
    }
}

#[cfg(test)]
mod tests;
