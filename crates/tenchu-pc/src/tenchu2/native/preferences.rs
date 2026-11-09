use crate::{
    controls::Controls,
    settings::{Settings, library::Manager},
    touch,
};

const TABS: [&str; 6] = [
    "General",
    "Display",
    "Textures",
    "Post-processing",
    "Controls",
    "Integration",
];

pub fn graphics(settings: &Settings) -> crate::settings::Graphics {
    let settings = settings.rendering();
    let g = &settings.graphics;
    crate::settings::Graphics {
        fog: g.fog,
        fog_distance: g.fog_distance,
        textures: g.textures,
        shading: g.shading,
        exposure: g.exposure,
        bilinear: g.bilinear,
        mipmaps: g.mipmaps,
        anisotropy: g.anisotropy,
        magnification: g.magnification,
        surface_detail: g.surface_detail,
        surface_strength: g.surface_strength,
        asset_pack: g.asset_pack,
        upscale: g.upscale,
        samples: g.samples,
        post: g.post.clone(),
        ..Default::default()
    }
}

#[derive(Default)]
pub struct Menu {
    pub open: bool,
    pub diagnostics: bool,
    pub fast_forward: bool,
    page: usize,
    error: Option<String>,
}

pub struct State<'a> {
    pub settings: &'a mut Settings,
    pub paused: &'a mut bool,
    pub controls: &'a mut Controls,
    pub presets: &'a mut Manager,
    pub samples: &'a [u32],
}

#[derive(Default)]
pub struct Action {
    pub leave: bool,
    pub select_mission: bool,
}

impl Menu {
    pub fn cutscene_controls(&mut self, ctx: &egui::Context, cinematic: bool) {
        if !cinematic {
            self.fast_forward = false;
            return;
        }
        if !self.open {
            egui::Area::new("jp-fast-forward".into())
                .anchor(egui::Align2::RIGHT_TOP, [-12.0, 42.0])
                .show(ctx, |ui| {
                    ui.spacing_mut().interact_size.y = 36.0;
                    ui.toggle_value(&mut self.fast_forward, "Fast-forward · Backspace");
                });
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut State<'_>) -> Action {
        let State {
            settings,
            paused,
            controls,
            presets,
            samples,
        } = state;
        let mut action = Action::default();
        if !self.open {
            return action;
        }
        if !controls.editor.open && ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
            let direction = if ctx.input(|i| i.modifiers.shift) {
                TABS.len() - 1
            } else {
                1
            };
            self.page = (self.page + direction) % TABS.len();
        }
        let screen = ctx.screen_rect();
        let mut window = egui::Window::new("Tenchu 2 settings · F1")
            .open(&mut self.open)
            .default_size([620.0, 500.0])
            .min_width(300.0_f32.min(screen.width() - 24.0))
            .max_width(screen.width() - 24.0)
            .max_height(screen.height() - 80.0)
            .resizable(true)
            .collapsible(false)
            .vscroll(true);
        if cfg!(target_os = "android") {
            window = window
                .fixed_pos([12.0, 64.0])
                .fixed_size([screen.width() - 40.0, (screen.height() - 112.0).max(120.0)]);
        }
        window.show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (page, name) in TABS.iter().enumerate() {
                    ui.selectable_value(&mut self.page, page, *name);
                }
            });
            ui.separator();
            ui.small("Settings save automatically and are shared with Gaisen. Integration lists features still unavailable in native JP.");
            if settings.retro { ui.label("Retro comparison is on. F8 restores your selected graphics."); }
            match self.page {
                0 => {
                    ui.checkbox(paused, "Pause");
                    ui.checkbox(&mut settings.pause_in_menu, "Pause while settings are open");
                    ui.checkbox(&mut settings.pause_on_defocus, "Pause when unfocused");
                    ui.checkbox(&mut settings.music_while_paused, "Music while paused")
                        .on_hover_text("Cutscene dialogue always pauses with playback to stay synchronized.");
                    ui.checkbox(&mut settings.show_fps, "Show FPS");
                    ui.add(egui::Slider::new(&mut settings.volume, 0..=100).text("Volume"));
                    ui.separator();
                    super::transport::speed_control(ui, &mut settings.cutscene_speed);
                    ui.separator();
                    action.select_mission = ui.button("Original JP mission selection").clicked();
                    action.leave = ui.button("Return to quick launcher · F6").clicked();
                }
                1 => {
                    let selected = presets.library.current(settings);
                    let choices = presets.library.choices();
                    egui::ComboBox::from_label("Visual preset")
                        .selected_text(choices.iter().find(|(c, _)| Some(*c) == selected).map_or("Custom", |(_, label)| label))
                        .show_ui(ui, |ui| {
                            for (choice, label) in choices {
                                if ui.selectable_label(Some(choice) == selected, label).clicked() {
                                    self.error = presets.library.apply(choice, settings).err();
                                }
                            }
                        });
                    ui.small("Presets apply the supported graphics below. They do not enable unconnected JP features.");
                    if ui.button("Presets / save as…").clicked() { presets.open = true; }
                    ui.checkbox(&mut settings.retro, "Retro comparison · F8");
                    ui.separator();
                    if !cfg!(target_os = "android") {
                        ui.checkbox(&mut settings.fullscreen, "Fullscreen");
                    }
                    ui.checkbox(&mut settings.vsync, "Vertical sync");
                    ui.checkbox(&mut settings.show_hud, "Show game HUD");
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Edge anti-aliasing");
                        for &value in samples.iter() {
                            ui.selectable_value(&mut settings.graphics.samples, value, format!("{value}×"));
                        }
                    });
                    ui.checkbox(&mut settings.graphics.fog, "Original distance shading / culling");
                    ui.add_enabled(settings.graphics.fog, egui::Slider::new(&mut settings.graphics.fog_distance, 25..=400).text("View distance %"));
                    ui.checkbox(&mut settings.graphics.textures, "Textures");
                    ui.checkbox(&mut settings.graphics.shading, "Original shading");
                    ui.add(egui::Slider::new(&mut settings.graphics.exposure, 25..=300).text("Exposure %"));
                }
                2 => {
                    ui.checkbox(&mut settings.graphics.asset_pack, "Replacement artwork");
                    if let Some(path) = crate::visuals::pack::configured_tenchu2_root() {
                        ui.label(format!("Tenchu 2 artwork: {}", path.display()));
                    } else {
                        ui.label("Install tenchu2-hd.tar.zst beside the game, or import it in the Android launcher. The Gaisen pack does not contain Tenchu 2 artwork.");
                    }
                    ui.hyperlink_to("Download Tenchu 2 artwork", "https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/tag/tenchu2-artwork-v2");
                    ui.separator();
                    let mut filtering = settings.graphics.filtering();
                    choices(ui, "Texture filtering", &mut filtering, &[(0,"Nearest"), (1,"Bilinear"), (2,"Mipmaps"), (3,"Anisotropic 2×"), (4,"Anisotropic 4×"), (5,"Anisotropic 8×"), (6,"Anisotropic 16×")]);
                    settings.graphics.set_filtering(filtering);
                    choices(ui, "Original texture upscale", &mut settings.graphics.upscale, &[(1,"1×"),(2,"2×"),(4,"4×")]);
                    choices(ui, "Texture magnification", &mut settings.graphics.magnification, &[(0,"Standard"),(1,"Sharp bilinear"),(2,"Cubic"),(3,"Pixel art")]);
                    choices(ui, "Surface relief", &mut settings.graphics.surface_detail, &[(0,"Off"),(1,"Shaded"),(2,"Parallax")]);
                    ui.add_enabled(settings.graphics.surface_detail != 0, egui::Slider::new(&mut settings.graphics.surface_strength, 0..=100).text("Surface strength %"));
                }
                3 => {
                    let post = &mut settings.graphics.post;
                    ui.checkbox(&mut post.enabled, "Post-processing");
                    ui.add_enabled_ui(post.enabled, |ui| {
                        ui.checkbox(&mut post.filmic, "Filmic tone mapping");
                        ui.add(egui::Slider::new(&mut post.bloom, 0..=100).text("Bloom %"));
                        ui.add(egui::Slider::new(&mut post.bloom_threshold, 25..=300).text("Bloom threshold %"));
                        ui.add(egui::Slider::new(&mut post.occlusion, 0..=100).text("Ambient occlusion %"));
                        ui.add(egui::Slider::new(&mut post.occlusion_radius, 2..=20).text("Occlusion radius"));
                        ui.add(egui::Slider::new(&mut post.contrast, 50..=150).text("Contrast %"));
                        ui.add(egui::Slider::new(&mut post.saturation, 0..=200).text("Saturation %"));
                        ui.add(egui::Slider::new(&mut post.temperature, -50..=50).text("Temperature"));
                        ui.add(egui::Slider::new(&mut post.gamma, 50..=150).text("Gamma %"));
                        ui.add(egui::Slider::new(&mut post.vignette, 0..=100).text("Vignette %"));
                        ui.add(egui::Slider::new(&mut post.sharpen, 0..=100).text("Sharpen %"));
                    });
                    if ui.button("Reset post-processing").clicked() { *post = Default::default(); }
                }
                4 => {
                    if ui.button("Remap keyboard / controller…").clicked() { controls.editor.open = true; controls.clear(); }
                    ui.label("JP controls: Triangle uses items; Circle guards; L2/R2 cycle items. F3 pauses, F4 steps one tick.");
                    ui.separator();
                    ui.label("Touch controls");
                    ui.horizontal_wrapped(|ui| {
                        for (value, label) in [(touch::Visibility::Auto,"Automatic"),(touch::Visibility::Always,"Always"),(touch::Visibility::Hidden,"Hidden")] {
                            ui.selectable_value(&mut settings.touch.visibility, value, label);
                        }
                    });
                    ui.add(egui::Slider::new(&mut settings.touch.size, 70..=140).text("Size %"));
                    ui.add(egui::Slider::new(&mut settings.touch.opacity, 15..=90).text("Opacity %"));
                    ui.checkbox(&mut settings.touch.left_handed, "Mirror gameplay controls");
                    if ui.button("Reset touch controls").clicked() { settings.touch = Default::default(); }
                }
                _ => {
                    ui.label("Native JP uses Tenchu 2 gameplay and assets. The Gaisen scenery sandbox has the Gaisen enhancements but uses Gaisen gameplay.");
                    ui.label("Not connected to native JP yet: modern lights/shadows, weather, modern blood and water effects, hair/cloth physics, model swapping, camera-relative controls, rewind, randomizer, gameplay cheats and performance benchmarks.");
                    ui.label("Those settings remain available in Gaisen and its scenery sandbox. Changing them there does not enable them here.");
                    ui.checkbox(&mut self.diagnostics, "Show integration diagnostics");
                }
            }
            if let Some(error) = &self.error { ui.colored_label(egui::Color32::LIGHT_RED, error); }
        });
        if let Some(choice) = presets.show(ctx, settings) {
            self.error = presets.library.apply(choice, settings).err();
        }
        controls.show_tenchu2_bindings(ctx);
        action
    }
}

fn choices(ui: &mut egui::Ui, label: &str, current: &mut u16, values: &[(u16, &str)]) {
    egui::ComboBox::from_label(label)
        .selected_text(
            values
                .iter()
                .find(|(value, _)| value == current)
                .map_or("Custom", |(_, name)| *name),
        )
        .show_ui(ui, |ui| {
            for &(value, name) in values {
                ui.selectable_value(current, value, name);
            }
        });
}
