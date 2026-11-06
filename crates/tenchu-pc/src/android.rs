use jni::{JValue, JavaVM, jni_sig, jni_str, objects::JObject};
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};
use tenchu_assets::Result;
use winit::platform::android::activity::AndroidApp;

static DATA: OnceLock<PathBuf> = OnceLock::new();
static APP: OnceLock<AndroidApp> = OnceLock::new();

pub fn data_dir() -> Result<&'static Path> {
    DATA.get()
        .map(PathBuf::as_path)
        .ok_or_else(|| "Android data directory unavailable".into())
}

fn activity(
    call: impl FnOnce(&mut jni::Env<'_>, &JObject<'_>) -> jni::errors::Result<()>,
) -> Result<()> {
    let app = APP.get().ok_or("Android activity unavailable")?;
    // AndroidApp owns the activity reference; the application context is not an Activity.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        let activity = unsafe { env.as_cast_raw::<JObject>(&raw)? };
        call(env, &activity)
    })
    .map_err(|e| {
        eprintln!("Android activity call failed: {e}");
        e.to_string()
    })
}

pub fn show_error(error: &str) {
    eprintln!("Tenchu: {error}");
    if let Err(error) = activity(|env, activity| {
        let text = env.new_string(error)?;
        env.call_method(
            activity,
            jni_str!("showError"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[JValue::Object(&text)],
        )?;
        Ok(())
    }) {
        eprintln!("Android error dialog: {error}");
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    let _ = APP.set(app.clone());
    let result = (|| {
        let root = app
            .internal_data_path()
            .ok_or("Android data directory unavailable")?;
        let _ = DATA.set(root.clone());
        std::env::set_current_dir(&root).map_err(|e| e.to_string())?;
        let disc = std::fs::read_to_string(root.join("disc-path.txt"))
            .map_err(|e| format!("Import your disc in the launcher: {e}"))?;
        let path = PathBuf::from(disc.trim());
        let assets = crate::startup::open(&path)?;
        crate::window::run(
            assets,
            Default::default(),
            &path,
            None,
            None,
            crate::window::Startup {
                android: Some(app),
                ..Default::default()
            },
        )
    })();
    if let Err(error) = result {
        show_error(&error);
    }
}

pub fn import(kind: &str) -> Result<()> {
    file_request(kind, "", "", false)
}

pub fn export(name: &str, bytes: &[u8]) -> Result<()> {
    let path = data_dir()?.join("exports").join(name);
    crate::storage::atomic_write(&path, bytes)?;
    file_request("export", &path.to_string_lossy(), name, true)
}

fn file_request(kind: &str, path: &str, name: &str, export: bool) -> Result<()> {
    activity(|env, activity| {
        let kind = env.new_string(kind)?;
        let path = env.new_string(path)?;
        let name = env.new_string(name)?;
        env.call_method(
            activity,
            jni_str!("fileRequest"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Z)V"),
            &[
                JValue::Object(&kind),
                JValue::Object(&path),
                JValue::Object(&name),
                JValue::Bool(export),
            ],
        )?;
        Ok(())
    })
}

#[derive(serde::Deserialize)]
pub struct FileResponse {
    pub kind: String,
    pub path: Option<PathBuf>,
    pub error: Option<String>,
}

pub fn poll_file() -> Option<Result<FileResponse>> {
    let path = data_dir().ok()?.join("file-response.json");
    match std::fs::read(&path) {
        Ok(bytes) => {
            let _ = std::fs::remove_file(path);
            Some(serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => Some(Err(error.to_string())),
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_tenchu_port_LauncherActivity_discFiles<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    path: jni::objects::JString<'local>,
) -> jni::objects::JObjectArray<'local, jni::objects::JString<'local>> {
    env.with_env(|env| -> jni::errors::Result<_> {
        let read = || -> Result<Vec<String>> {
            let cue = std::fs::read_to_string(path.to_string()).map_err(|e| e.to_string())?;
            tenchu_assets::cue::data_track(&cue)?;
            Ok(tenchu_assets::cue::binary_files(&cue)?
                .into_iter()
                .map(str::to_owned)
                .collect())
        };
        let files = match read() {
            Ok(files) => files,
            Err(error) => {
                return env
                    .throw_new(
                        jni_str!("java/io/IOException"),
                        jni::strings::JNIString::from(error),
                    )
                    .map(|()| jni::objects::JObjectArray::<jni::objects::JString>::null());
            }
        };
        let array = jni::objects::JObjectArray::<jni::objects::JString>::new(
            env,
            files.len(),
            jni::objects::JString::null(),
        )?;
        for (index, file) in files.iter().enumerate() {
            let name = env.new_string(file)?;
            array.set_element(env, index, &name)?;
        }
        Ok(array)
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_tenchu_port_LauncherActivity_validateDisc<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    path: jni::objects::JString<'local>,
) -> jni::objects::JString<'local> {
    env.with_env(|env| -> jni::errors::Result<_> {
        match crate::startup::open(Path::new(&path.to_string())) {
            Ok(_) => Ok(jni::objects::JString::null()),
            Err(error) => env.new_string(error),
        }
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_tenchu_port_LauncherActivity_prepareArtwork<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    root: jni::objects::JString<'local>,
    path: jni::objects::JString<'local>,
) -> jni::objects::JString<'local> {
    env.with_env(|env| -> jni::errors::Result<_> {
        let _ = DATA.set(PathBuf::from(root.to_string()));
        match crate::visuals::pack::unpack(Path::new(&path.to_string())) {
            Ok(_) => Ok(jni::objects::JString::null()),
            Err(error) => env.new_string(error),
        }
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
}

struct TextEditor {
    active: bool,
    focus: Option<egui::Id>,
    result: Option<Option<String>>,
}

static TEXT_EDITOR: std::sync::Mutex<TextEditor> = std::sync::Mutex::new(TextEditor {
    active: false,
    focus: None,
    result: None,
});

pub fn text_events(ctx: &egui::Context) -> Option<Vec<egui::Event>> {
    let mut editor = TEXT_EDITOR.lock().unwrap();
    let text = editor.result.take()?;
    editor.active = false;
    if let Some(id) = editor.focus.take() {
        ctx.memory_mut(|m| m.request_focus(id));
    }
    let mut events = Vec::new();
    if let Some(text) = text {
        for pressed in [true, false] {
            events.push(egui::Event::Key {
                key: egui::Key::A,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            });
        }
        // Backspace also permits replacing a field with an empty string.
        for pressed in [true, false] {
            events.push(egui::Event::Key {
                key: egui::Key::Backspace,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: Default::default(),
            });
        }
        events.push(egui::Event::Paste(text));
    }
    Some(events)
}

pub fn platform_output(ctx: &egui::Context, output: &mut egui::PlatformOutput) {
    let mut editor = TEXT_EDITOR.lock().unwrap();
    let text = if output.ime.take().is_some() && !editor.active {
        output
            .events
            .iter()
            .rev()
            .find_map(|e| e.widget_info().current_text_value.clone())
    } else {
        None
    };
    if text.is_some() {
        editor.active = true;
        editor.focus = ctx.memory(|m| m.focused());
    }
    drop(editor);
    if text.is_none() && output.commands.is_empty() {
        return;
    }
    let result = activity(|env, activity| {
        if let Some(value) = text {
            let value = env.new_string(value)?;
            env.call_method(
                activity,
                jni_str!("editText"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&value)],
            )?;
        }
        for command in &output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                let text = env.new_string(text)?;
                env.call_method(
                    activity,
                    jni_str!("copyText"),
                    jni_sig!("(Ljava/lang/String;)V"),
                    &[JValue::Object(&text)],
                )?;
            }
        }
        Ok(())
    });
    if let Err(error) = result {
        TEXT_EDITOR.lock().unwrap().active = false;
        eprintln!("Android text input: {error}");
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_tenchu_port_GameActivity_submitText<'local>(
    mut env: jni::EnvUnowned<'local>,
    _class: jni::objects::JClass<'local>,
    accepted: jni::sys::jboolean,
    text: jni::objects::JString<'local>,
) {
    env.with_env(|_| -> jni::errors::Result<()> {
        TEXT_EDITOR.lock().unwrap().result = Some(accepted.then(|| text.to_string()));
        Ok(())
    })
    .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

static BACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn take_back() -> bool {
    BACK.swap(false, std::sync::atomic::Ordering::Relaxed)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_org_tenchu_port_GameActivity_back(
    _env: jni::EnvUnowned<'_>,
    _class: jni::objects::JClass<'_>,
) {
    BACK.store(true, std::sync::atomic::Ordering::Relaxed);
}
