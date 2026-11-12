use serde_json::{Value, json};
use tenchu_assets::Result;

pub(super) fn platform() -> Value {
    let mut info = json!({
        "os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "pointer_bits": usize::BITS, "byte_order": if cfg!(target_endian = "little") { "little" } else { "big" },
        "available_threads": std::thread::available_parallelism().ok().map(|n| n.get()),
    });
    #[cfg(target_os = "android")]
    {
        info["platform"] = json!("Android NativeActivity");
    }
    #[cfg(target_os = "linux")]
    {
        info["kernel"] = std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .ok()
            .map(|s| s.trim().to_owned())
            .into();
        if let Ok(release) = std::fs::read_to_string("/etc/os-release") {
            info["distribution"] = release
                .lines()
                .filter_map(|line| {
                    let (key, value) = line.split_once('=')?;
                    ["ID", "VERSION_ID", "PRETTY_NAME"]
                        .contains(&key)
                        .then(|| (key.into(), value.trim_matches('"').into()))
                })
                .collect::<serde_json::Map<String, Value>>()
                .into();
        }
        info["session_type"] = std::env::var("XDG_SESSION_TYPE").ok().into();
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        info["version"] = std::process::Command::new("cmd")
            .args(["/D", "/C", "ver"])
            .creation_flags(0x08000000)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .into();
    }
    #[cfg(target_os = "macos")]
    {
        info["version"] = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .into();
    }
    #[cfg(target_arch = "wasm32")]
    {
        info["browser"] =
            serde_json::from_str(&crate::web::diagnostic_platform()).unwrap_or(Value::Null);
    }
    info
}

type SaveFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<String>>>>>;

#[derive(Default)]
pub struct Save {
    pending: Option<SaveFuture>,
}

impl Save {
    pub fn active(&self) -> bool {
        self.pending.is_some()
    }

    pub fn start(&mut self, name: String, bytes: Vec<u8>) {
        if !self.active() {
            self.pending = Some(Box::pin(save(name, bytes)));
        }
    }

    pub fn poll(&mut self) -> Option<Result<Option<String>>> {
        // Both window loops redraw while paused, including while a native dialog is open.
        let context = &mut std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(result) = self.pending.as_mut()?.as_mut().poll(context) else {
            return None;
        };
        self.pending = None;
        Some(result)
    }
}

async fn save(name: String, bytes: Vec<u8>) -> Result<Option<String>> {
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
    {
        let Some(file) = rfd::AsyncFileDialog::new()
            .set_title("Save diagnostic report")
            .add_filter("Diagnostic report (JSON)", &["json"])
            .set_file_name(&name)
            .save_file()
            .await
        else {
            return Ok(None);
        };
        let path = file.path();
        crate::storage::atomic_write(path, &bytes)?;
        Ok(Some(format!(
            "Diagnostic report saved to {}",
            path.display()
        )))
    }
    #[cfg(target_arch = "wasm32")]
    {
        crate::web::download_report(
            &name,
            std::str::from_utf8(&bytes).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("Diagnostic download failed: {e:?}"))?;
        Ok(Some("Diagnostic report sent to browser downloads".into()))
    }
    #[cfg(target_os = "android")]
    {
        crate::android::export(&name, &bytes)?;
        Ok(Some("Choose where to save the report".into()))
    }
}

pub fn renderer(
    adapter: &wgpu::AdapterInfo,
    window: &winit::window::Window,
    config: &wgpu::SurfaceConfiguration,
    renderer: &crate::renderer::Renderer,
) -> Value {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let window_backend = match window.window_handle().map(|h| h.as_raw()) {
        Ok(RawWindowHandle::Xlib(_) | RawWindowHandle::Xcb(_)) => "X11",
        Ok(RawWindowHandle::Wayland(_)) => "Wayland",
        Ok(RawWindowHandle::Win32(_)) => "Win32",
        Ok(RawWindowHandle::AppKit(_)) => "AppKit",
        Ok(RawWindowHandle::AndroidNdk(_)) => "Android",
        Ok(
            RawWindowHandle::Web(_)
            | RawWindowHandle::WebCanvas(_)
            | RawWindowHandle::WebOffscreenCanvas(_),
        ) => "Web",
        _ => "Unknown",
    };
    serde_json::json!({
        "adapter": {"name": adapter.name, "vendor_id": adapter.vendor,
            "device_id": adapter.device, "device_type": format!("{:?}", adapter.device_type),
            "backend": format!("{:?}", adapter.backend),
            "driver": adapter.driver, "driver_info": adapter.driver_info},
        "window_pixels": [config.width, config.height], "render_pixels": renderer.size(),
        "window_backend": window_backend,
        "scale_factor": window.scale_factor(),
        "present_mode": format!("{:?}", config.present_mode),
        "surface_format": format!("{:?}", config.format),
        "supported_msaa": renderer.sample_counts,
        "gpu_timestamps_supported": renderer.timings.is_some(),
        "gpu_profiling_active": renderer.profiling,

    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_dialog_yields_until_completion_then_releases_pause() {
        use std::{cell::RefCell, rc::Rc, task::Poll};
        for result in [
            Ok(Some("saved".to_owned())),
            Ok(None),
            Err("write failed".to_owned()),
        ] {
            let reply = Rc::new(RefCell::new(None));
            let future_reply = reply.clone();
            let mut save = Save {
                pending: Some(Box::pin(std::future::poll_fn(move |_| {
                    future_reply
                        .borrow_mut()
                        .take()
                        .map_or(Poll::Pending, Poll::Ready)
                }))),
            };
            for _ in 0..120 {
                assert!(save.active());
                assert!(save.poll().is_none());
            }
            *reply.borrow_mut() = Some(result.clone());
            assert_eq!(save.poll(), Some(result));
            assert!(!save.active());
            assert!(save.poll().is_none());
        }
    }
}
