use crate::{capture::Readback, demo::Player, renderer::Renderer};
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tenchu_assets::Result;

pub const SIZE: [u32; 2] = [3840, 2160];
pub const FPS: u64 = 60;

pub struct Export {
    pub texture: wgpu::Texture,
    pub frames: u64,
    pub total: u64,
    readback: Readback,
    encoder: Child,
    input: Option<ChildStdin>,
    files: Files,
    chapters: Chapters,
    started: Instant,
    reported: Instant,
}

struct Files {
    directory: PathBuf,
    output: PathBuf,
    chapters: PathBuf,
}

struct Chapters {
    metadata: String,
    description: String,
    starts: Vec<u64>,
    frames: u64,
}

impl Export {
    pub fn new(player: &Player, renderer: &Renderer, format: wgpu::TextureFormat) -> Result<Self> {
        let output = player.video.as_ref().ok_or("No video destination")?;
        if !output
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("mp4"))
        {
            return Err("--video needs an .mp4 destination".into());
        }
        let pixel_format = match format {
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => "rgba",
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => "bgra",
            _ => return Err(format!("Video capture format {format:?} is unsupported")),
        };
        let files = Files::new(output)?;
        let chapters = Chapters::new(player);
        fs::write(
            files.directory.join("chapters.ffmetadata"),
            &chapters.metadata,
        )
        .map_err(|e| e.to_string())?;
        fs::write(files.directory.join("chapters.txt"), &chapters.description)
            .map_err(|e| e.to_string())?;
        let log = File::create(files.directory.join("encoder.log")).map_err(|e| e.to_string())?;
        let ffmpeg = std::env::var_os("TENCHU_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
        let mut encoder = Command::new(ffmpeg)
            .args(["-hide_banner", "-loglevel", "warning", "-nostdin"])
            .args(["-f", "rawvideo", "-pixel_format", pixel_format])
            .args(["-video_size", &format!("{}x{}", SIZE[0], SIZE[1])])
            .args(["-framerate", "60", "-i", "pipe:0", "-f", "ffmetadata", "-i"])
            .arg(files.directory.join("chapters.ffmetadata"))
            .args([
                "-map",
                "0:v:0",
                "-map_metadata",
                "1",
                "-map_chapters",
                "1",
                "-an",
            ])
            .args([
                "-vf",
                "scale=in_range=full:out_range=tv:out_color_matrix=bt709,format=yuv420p,setparams=range=limited:color_primaries=bt709:color_trc=bt709:colorspace=bt709,setsar=1",
                "-c:v",
                "libx264",
                "-preset",
                "fast",
                "-crf",
                "16",
                "-maxrate",
                "68M",
                "-bufsize",
                "136M",
                "-profile:v",
                "high",
                "-level:v",
                "5.2",
                "-g",
                "30",
                "-bf",
                "2",
                "-x264-params",
                "open-gop=0",
                "-fps_mode",
                "passthrough",
                "-color_range",
                "tv",
                "-colorspace",
                "bt709",
                "-color_trc",
                "bt709",
                "-color_primaries",
                "bt709",
                "-movflags",
                "+faststart+negative_cts_offsets",
                "-use_editlist",
                "0",
                "-video_track_timescale",
                "60000",
                "-threads",
            ])
            .arg(
                std::thread::available_parallelism()
                    .map_or(4, |n| n.get().min(16))
                    .to_string(),
            )
            .arg("-force_key_frames")
            .arg(chapters.starts.iter().map(|frame| (frame / FPS).to_string()).collect::<Vec<_>>().join(","))
            .args(["-f", "mp4"])
            .arg(files.directory.join("video.mp4"))
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .map_err(|e| {
                format!("Cannot start FFmpeg: {e}. Use the Nix shell or TENCHU_FFMPEG.")
            })?;
        let input = encoder.stdin.take();
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offline video"),
            size: wgpu::Extent3d {
                width: SIZE[0],
                height: SIZE[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let readback = Readback::new(renderer, &texture);
        eprintln!(
            "Exporting {} frames at 3840×2160 / 60 fps to {}",
            chapters.frames,
            output.display()
        );
        Ok(Self {
            texture,
            frames: 0,
            total: chapters.frames,
            readback,
            encoder,
            input,
            files,
            chapters,
            started: Instant::now(),
            reported: Instant::now(),
        })
    }

    pub fn write(&mut self, renderer: &Renderer, player: &Player) -> Result<()> {
        if self.frames != self.chapters.starts[player.index] + player.frame {
            return Err("Video timeline skipped or repeated a frame".into());
        }
        let input = self.input.as_mut().ok_or("Video encoder is closed")?;
        self.readback
            .read(renderer, &self.texture, |pixels, stride| {
                let width = SIZE[0] as usize * 4;
                if stride == width {
                    input.write_all(pixels).map_err(|e| e.to_string())
                } else {
                    for row in pixels.chunks_exact(stride) {
                        input.write_all(&row[..width]).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                }
            })
            .map_err(|e| format!("Video frame {}: {e}\n{}", self.frames, self.files.log()))?;
        self.frames += 1;
        if self.reported.elapsed().as_secs() >= 10 {
            let rate = self.frames as f64 / self.started.elapsed().as_secs_f64();
            eprintln!(
                "Video {}/{} frames ({:.1}%) · {:.1} rendered fps · {}",
                self.frames,
                self.total,
                self.frames as f64 * 100.0 / self.total as f64,
                rate,
                player.clip().id
            );
            self.reported = Instant::now();
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<()> {
        if self.frames != self.total {
            return Err(format!(
                "Video interrupted after {} of {} frames",
                self.frames, self.total
            ));
        }
        self.input.take();
        let status = self.encoder.wait().map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("FFmpeg {status}: {}", self.files.log()));
        }
        // Publishing with a hard link refuses to replace a file created during export.
        fs::hard_link(self.files.directory.join("video.mp4"), &self.files.output)
            .map_err(|e| e.to_string())?;
        fs::hard_link(
            self.files.directory.join("chapters.txt"),
            &self.files.chapters,
        )
        .map_err(|e| e.to_string())?;
        eprintln!(
            "Saved {} ({} frames, {}s)\nYouTube description chapters: {}",
            self.files.output.display(),
            self.frames,
            self.frames / FPS,
            self.files.chapters.display()
        );
        Ok(())
    }
}

impl Drop for Export {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.encoder.kill();
        let _ = self.encoder.wait();
    }
}

impl Files {
    fn new(output: &Path) -> Result<Self> {
        let chapters = output.with_extension("chapters.txt");
        for path in [output, &chapters] {
            if path.exists() {
                return Err(format!("Refusing to overwrite {}", path.display()));
            }
        }
        let parent = output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let directory = parent.join(format!(".tenchu-video-{}-{stamp}", std::process::id()));
        fs::create_dir(&directory).map_err(|e| e.to_string())?;
        Ok(Self {
            directory,
            output: output.to_owned(),
            chapters,
        })
    }

    fn log(&self) -> String {
        fs::read_to_string(self.directory.join("encoder.log")).unwrap_or_default()
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

impl Chapters {
    fn new(player: &Player) -> Self {
        let mut result = Self {
            metadata: ";FFMETADATA1\ntitle=Tenchu: Shinobi Gaisen — Native PC Showcase\n".into(),
            description: String::new(),
            starts: Vec::new(),
            frames: 0,
        };
        for &index in &player.playlist {
            let clip = &player.catalog[index];
            let title = clip.title.replace(['\n', '\r'], " ");
            let start = result.frames;
            result.starts.push(start);
            result.frames += u64::from(clip.seconds) * FPS;
            let seconds = start / FPS;
            let timestamp = if seconds >= 3600 {
                format!(
                    "{}:{:02}:{:02}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                )
            } else {
                format!("{:02}:{:02}", seconds / 60, seconds % 60)
            };
            result
                .description
                .push_str(&format!("{timestamp} {title}\n"));
            let escaped: String = title
                .chars()
                .flat_map(|c| {
                    matches!(c, '\\' | '=' | ';' | '#')
                        .then_some('\\')
                        .into_iter()
                        .chain([c])
                })
                .collect();
            result.metadata.push_str(&format!(
                "\n[CHAPTER]\nTIMEBASE=1/60\nSTART={start}\nEND={}\ntitle={escaped}\n",
                result.frames
            ));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapters_follow_selected_order_and_exact_frame_boundaries() {
        let mut player = Player::parse(&["--clips".into(), "moon,shadows,moon".into()]).unwrap();
        player.catalog[player.playlist[0]].title = "Moon = light; #1\\2\nagain".into();
        let chapters = Chapters::new(&player);
        assert_eq!(chapters.starts, [0, 840, 1560]);
        assert_eq!(chapters.frames, 2400);
        assert!(
            chapters
                .description
                .starts_with("00:00 Moon = light; #1\\2 again\n00:14 ")
        );
        assert!(chapters.description.contains("00:26 Moon"));
        assert!(
            chapters
                .metadata
                .contains("title=Moon \\= light\\; \\#1\\\\2 again")
        );
        assert!(chapters.metadata.contains("START=1560\nEND=2400"));
    }
}
