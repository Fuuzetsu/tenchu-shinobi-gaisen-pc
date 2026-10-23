use super::*;
use egui::{Align2, Color32, FontId, Pos2, Rect, Vec2 as EVec2};
use glam::{Vec2, Vec3};
use raster::{Blend, Sprite, Vertex};

const OPTIONS: [&str; 10] = [
    "Records",
    "Difficulty",
    "Sound: stereo / mono",
    "Music and effects volume",
    "Sound test",
    "Controller layout",
    "Controller vibration",
    "Load / save progress",
    "Automatic saving",
    "Language",
];
const ORIGINAL_OPTIONS: [&str; 10] = [
    "過去記録",
    "難易度",
    "サウンドモード",
    "音量調整",
    "演奏試聴",
    "コントローラー",
    "振動機能",
    "メモリーカード",
    "自動記録",
    "言語選択",
];

impl Frontend {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        campaign: &Campaign,
        interactive: bool,
        settings: &crate::settings::Settings,
    ) -> Option<Action> {
        let high_definition = !matches!(self.page, Page::Movie(_))
            && (settings.sharp_text || settings.graphics.asset_pack && !self.artwork.is_empty());
        let artwork = if settings.graphics.asset_pack {
            self.artwork.clone()
        } else {
            Arc::default()
        };
        if self.high_definition != high_definition
            || self.artwork_enabled != settings.graphics.asset_pack
            || self.canvas.sharp_text != settings.sharp_text
        {
            self.high_definition = high_definition;
            self.artwork_enabled = settings.graphics.asset_pack;
            self.canvas.sharp_text = settings.sharp_text;
            self.dirty = true;
        }
        if self.dirty {
            self.canvas.frame =
                high_definition.then(|| draw::Frame::new(self.canvas.size, artwork));
            self.paint(campaign);
            self.frame = self.canvas.frame.take().map(Arc::new);
            if !high_definition {
                let image =
                    egui::ColorImage::from_rgba_unmultiplied(self.canvas.size, &self.canvas.pixels);
                if let Some(texture) = &mut self.texture {
                    texture.set(image, egui::TextureOptions::NEAREST);
                } else {
                    self.texture = Some(ctx.load_texture(
                        "Original frontend",
                        image,
                        egui::TextureOptions::NEAREST,
                    ));
                }
            }
            self.dirty = false;
        }
        let screen = ctx.screen_rect();
        let height = (screen.height() - 60.0).min(screen.width() * 0.75).max(1.0);
        let size = EVec2::new(height * 4.0 / 3.0, height);
        let rect = Rect::from_center_size(screen.center() - EVec2::new(0.0, 30.0), size);
        let painter = ctx.layer_painter(egui::LayerId::background());
        painter.rect_filled(screen, 0.0, Color32::BLACK);
        if let Some(frame) = &self.frame {
            painter.add(egui_wgpu::Callback::new_paint_callback(
                rect,
                gpu::Paint(frame.clone()),
            ));
        } else if let Some(texture) = &self.texture {
            painter.image(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        crate::fonts::paint(ctx, rect, &self.canvas.text);
        let label = self.description(campaign);
        painter.text(
            egui::pos2(screen.center().x, screen.bottom() - 43.0),
            Align2::CENTER_CENTER,
            &label,
            if settings.sharp_text {
                crate::fonts::id(15.0)
            } else {
                FontId::proportional(15.0)
            },
            Color32::WHITE,
        );
        let help = if self.notice.is_empty() {
            "Arrows: select   Enter / South: confirm   Esc / East: back   F6: quick launcher   F1: PC settings"
        } else {
            &self.notice
        };
        painter.text(
            egui::pos2(screen.center().x, screen.bottom() - 20.0),
            Align2::CENTER_CENTER,
            help,
            FontId::proportional(13.0),
            Color32::LIGHT_GRAY,
        );
        if !interactive || !self.ready() {
            return None;
        }
        let mut action = None;
        egui::Area::new("frontend mouse".into())
            .order(egui::Order::Background)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                let response = ui.allocate_rect(rect, egui::Sense::click());
                if response.clicked()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    let p = (pos - rect.min) / rect.size();
                    let x = p.x * self.canvas.size[0] as f32 - self.canvas.size[0] as f32 / 2.0;
                    let y = p.y * 240.0 - 120.0;
                    action = self.click(x, y, campaign);
                }
            });
        action
    }

    fn click(&mut self, x: f32, y: f32, campaign: &Campaign) -> Option<Action> {
        match self.page {
            Page::Title => {
                if self.title.selection.is_some()
                    && (-45.0..45.0).contains(&x)
                    && (37.0..111.0).contains(&y)
                {
                    self.title.selection = Some(((y - 39.0) / 16.0).clamp(0.0, 3.0) as usize);
                }
            }
            Page::Character if !self.characters.confirming => {
                self.characters.select(usize::from(x >= 0.0), &self.tables)
            }
            Page::Options => {
                self.option_selection =
                    ((y + 85.0) / 34.0).clamp(0.0, 4.0) as usize + if x >= 0.0 { 5 } else { 0 };
            }
            Page::Choice(option) => {
                let n = if option == 9 {
                    4
                } else if option == 1 {
                    3
                } else {
                    2
                };
                let step = if n == 4 {
                    20.0
                } else if n == 3 {
                    28.0
                } else {
                    40.0
                };
                self.choice =
                    ((y / step + (n - 1) as f32 / 2.0).round()).clamp(0.0, (n - 1) as f32) as usize;
            }
            Page::Missions => {
                if y > 30.0 {
                    let layout = ((x + 102.0) / 120.0).clamp(0.0, 2.0) as usize;
                    if layout_available(campaign, self.mission, layout) {
                        self.layout = layout;
                    }
                } else {
                    self.mission = (1..=unlocked_mission(campaign))
                        .min_by(|&a, &b| {
                            let distance = |i| {
                                let [cx, cy] = self.tables.cursor_targets[i - 1];
                                (x - f32::from(cx)).powi(2) + (y - f32::from(cy)).powi(2)
                            };
                            distance(a).total_cmp(&distance(b))
                        })
                        .unwrap_or(1);
                    self.layout = 0;
                    self.cursor = self.tables.cursor_starts[self.mission - 1].map(f32::from);
                    self.dirty = true;
                    return None;
                }
            }
            Page::Card => self.choice = usize::from(y >= 0.0),
            Page::Records => {
                self.selection = usize::from(x > 80.0);
                self.dirty = true;
                return None;
            }
            Page::Volume => {
                self.selection = usize::from(y >= 0.0);
                self.dirty = true;
                return Some(if x >= 50.0 {
                    Action::Right
                } else {
                    Action::Left
                });
            }
            Page::Controls => {
                return Some(if x >= 0.0 {
                    Action::Right
                } else {
                    Action::Left
                });
            }
            _ => (),
        }
        self.dirty = true;
        Some(Action::Confirm)
    }

    fn description(&self, campaign: &Campaign) -> String {
        match self.page {
            Page::Title => self.title.selection.map_or("Tenchu: Shinobi Gaisen — press Enter".into(),|i| ["Begin mission", "Ninja training", "Mission editor", "Options"][i].into()),
            Page::Character => ["Select character — Rikimaru", "Select character — Ayame"][self.characters.selected].into(),
            Page::Missions => format!("Mission {}: {} — layout {}",self.mission,campaign.stages[stage_id(campaign,self.mission)].name,self.layout+1),
            Page::Options => OPTIONS[self.option_selection].into(),
            Page::Choice(option) => format!("{} — {}",OPTIONS[option], match option {
                1=>["Easy","Normal","Hard"][self.choice],2=>["Stereo","Mono"][self.choice],
                9=>["日本語 / Japanese","English","Français / French","Italiano / Italian"][self.choice],
                _=>["On","Off"][self.choice],
            }),
            Page::Card => if self.choice==0 {"Load saved campaign"}else{"Save current campaign"}.into(),
            Page::Volume => format!("Music {} / 127    Effects {} / 127 — Enter applies, Esc restores",self.preferences.music,self.preferences.effects),
            Page::Controls => format!("Controller layout {} — Enter applies, Esc cancels",self.choice+1),
            Page::SoundTest => format!("{} {} — left/right changes, Enter plays",if self.selection==0 {"Music"}else{"Effect"},self.sound_choices[self.selection]),
            Page::Records => format!("Records — {}",if self.selection==0 {"Rikimaru"}else{"Ayame"}),
            Page::Editor => "The mission editor is a separate TRIAL.EXE program; it has not been ported. Enter returns.".into(),
            Page::Movie(movie) => format!("{}{}",match movie {Movie::Opening=>"Opening",Movie::Training=>"Ninja training",Movie::Rikimaru=>"Rikimaru epilogue",Movie::Ayame=>"Ayame epilogue",Movie::Credits=>"Credits"},if movie.skippable(){" — Enter or Esc to skip"}else{""}),
            Page::Results => "Final results — Enter: main menu    R / F5: replay mission".into(),
        }
    }

    pub(super) fn paint(&mut self, campaign: &Campaign) {
        self.canvas.clear();
        match self.page {
            Page::Title => self.paint_title(),
            Page::Character => {
                self.canvas
                    .sprite(Sprite::new(&self.character_images[0], 0.0, 0.0));
                self.characters.draw(&mut self.canvas, &campaign.game.math);
                let selected = self.characters.selected;
                let pulse = self.ticks as f32 * 15.0 * std::f32::consts::TAU / 4096.0;
                self.canvas.sprite(
                    Sprite::new(
                        &self.character_images[2],
                        if selected == 0 { -72.0 } else { 72.0 },
                        -16.0,
                    )
                    .scale(if selected == 0 { 1.0 } else { -1.0 }, 1.0)
                    .light(128 + ((pulse.cos() + (pulse * 3.0).sin()) * 32.0) as i32)
                    .blend(Blend::Add),
                );
                self.canvas.label(
                    Sprite::new(&self.character_images[1], 0.0, -96.0).blend(Blend::Add),
                    match self.language {
                        Language::English => "CHARACTER",
                        Language::Japanese => "主人公の選択",
                        Language::French => "Personnage",
                        Language::Italian => "Personaggi",
                    },
                    Color32::WHITE,
                );
            }
            Page::Missions => self.paint_missions(campaign),
            Page::Movie(_) => {
                if let Some(movie) = &mut self.movie {
                    movie.paint(&mut self.canvas);
                }
            }
            Page::Results => self.paint_results(campaign),
            _ => self.paint_options(campaign),
        }
        if self.page != Page::Title && !matches!(self.page, Page::Movie(_)) {
            let step = if self.page == Page::Results { 4 } else { 8 };
            self.canvas.fade((128 - self.ticks as i32 * step).max(0));
        }
    }

    fn paint_title(&mut self) {
        let blood_count = self.title.frames[6].max(0) as usize / 3;
        for index in 0..16 {
            let color = self
                .tables
                .blood_order
                .iter()
                .position(|&i| i == index)
                .filter(|&i| i < blood_count)
                .map_or(0, |i| self.blood_colors[i]);
            self.title_images[1].recolor(index, color);
        }
        for image in &mut self.title_images[5..7] {
            if let Some(color) = image.palette_color(14) {
                image.recolor(
                    14,
                    (color & 0x7fff)
                        | if self.title.frames[2] >= 200 {
                            0x8000
                        } else {
                            0
                        },
                );
            }
        }
        let images = &self.title_images;
        let title = &self.title;
        let [logo, genre, seal, subtitle, kunai, copyright, blood] = title.frames;
        let image = &images[0];
        let point = |x: usize, y: usize| {
            let p = Vec2::new(x as f32 * 20.0 - 160.0, y as f32 * 15.0 - 120.0);
            let phase =
                (title.clock / 5 + 1) as f32 * (250.0 - p.length().floor()) * std::f32::consts::TAU
                    / 4096.0;
            p * (if x == 0 || x == 16 || y == 0 || y == 16 {
                1.0
            } else {
                1.0 + phase.cos() / 20.0
            }) + Vec2::new(160.0, 120.0)
        };
        let mut vertices = Vec::with_capacity(16 * 16 * 6);
        for y in 0..16 {
            for x in 0..16 {
                let v = |x, y| Vertex {
                    position: point(x, y),
                    uv: Vec2::new(x as f32 * 20.0, y as f32 * 15.0),
                    color: Vec3::splat(128.0),
                };
                for triangle in [
                    [v(x, y), v(x + 1, y), v(x, y + 1)],
                    [v(x + 1, y), v(x + 1, y + 1), v(x, y + 1)],
                ] {
                    vertices.extend(triangle);
                }
            }
        }
        self.canvas.image_triangles(image, &vertices, Blend::Opaque);
        let seals = |canvas: &mut Canvas| {
            let radius = (80 - (200 - seal) * 2) as f32;
            for i in 0..9 {
                let angle = (title.seal_angle as f32 + i as f32 * 4096.0 / 9.0)
                    * std::f32::consts::TAU
                    / 4096.0;
                canvas.sprite(
                    Sprite::new(
                        &images[5 + i % 2],
                        radius * angle.cos(),
                        radius * angle.sin() - 24.0,
                    )
                    .row(i / 2, if i % 2 == 0 { 5 } else { 4 })
                    .angle(angle + std::f32::consts::FRAC_PI_2)
                    .blend(Blend::Add),
                );
            }
        };
        if seal >= 200 {
            seals(&mut self.canvas);
        }
        if logo >= 0 {
            let scale = 1.0 + (128 - logo) as f32 * 40.0 / 4096.0;
            self.canvas.sprite(
                Sprite::new(&images[10], 15.0, -40.0)
                    .scale(scale, scale)
                    .light(logo)
                    .blend(Blend::Subtract),
            );
            if logo == 128 {
                self.canvas
                    .sprite(Sprite::new(&images[11], 125.0, 12.0).blend(Blend::Average));
            }
        }
        if subtitle >= 0 {
            let remaining = (30 - subtitle) as f32;
            self.canvas.sprite(
                Sprite::new(&images[2], -45.0 + remaining * 2.0, 23.0 - remaining * 2.0)
                    .angle(remaining * 16.0f32.to_radians())
                    .scale((remaining + 16.0) / 16.0, (remaining + 16.0) / 16.0),
            );
        }
        if genre >= 0 {
            for blend in [Blend::Average, Blend::Add] {
                self.canvas.sprite(
                    Sprite::new(&images[7], 0.0, -100.0)
                        .light(genre)
                        .blend(blend),
                );
            }
            self.canvas.sprite(
                Sprite::new(&images[9], 65.0, 23.0)
                    .light(genre)
                    .blend(Blend::Subtract),
            );
        }
        if kunai >= 0 {
            let remaining = (10 - kunai) as f32;
            self.canvas.sprite(Sprite::new(
                &images[8],
                -28.0 + remaining * 10.0,
                -33.0 - remaining * 15.0,
            ));
            if kunai == 10 {
                self.canvas
                    .sprite(Sprite::new(&images[4], -60.0, 23.0).blend(Blend::Subtract));
            }
        }
        if copyright >= 0 {
            for i in 0..2 {
                self.canvas.sprite(
                    Sprite::new(&images[12 + i], if i == 0 { -76.0 } else { 76.0 }, 87.0)
                        .light(copyright)
                        .blend(Blend::Add),
                );
            }
        }
        if blood >= 0 {
            self.canvas.sprite(Sprite::new(&images[1], -101.0, -84.0));
            self.canvas.sprite(Sprite::new(&images[3], -89.0, -65.0));
        }
        if (0..200).contains(&seal) {
            seals(&mut self.canvas);
        }
        if let Some(selected) = title.selection {
            for i in 0..4 {
                let row = [0, 1, 3, 2][i];
                let y = 47.0 + i as f32 * 16.0;
                self.canvas.sprite(
                    Sprite::new(&images[15], 0.0, y)
                        .row(row, 4)
                        .blend(Blend::Subtract),
                );
                if self.canvas.sharp_text {
                    self.canvas.label(
                        Sprite::new(&images[14], 0.0, y).row(row, 4),
                        ["任務開始", "忍術指南", "虎の巻", "各種設定"][i],
                        if i == selected {
                            Color32::from_rgb(255, 216, 145)
                        } else {
                            Color32::LIGHT_GRAY
                        },
                    );
                    continue;
                }
                self.canvas
                    .sprite(
                        Sprite::new(&images[14], 0.0, y)
                            .row(row, 4)
                            .blend(if i == selected {
                                Blend::Add
                            } else {
                                Blend::Quarter
                            }),
                    );
            }
        }
        self.canvas.fade(title.fade);
    }

    fn paint_missions(&mut self, campaign: &Campaign) {
        let images = &self.mission_images;
        self.canvas.sprite(Sprite::new(&images[0], 0.0, 0.0));
        self.canvas.sprite(
            Sprite::new(&images[28], self.cursor[0], self.cursor[1])
                .light(80 + (self.ticks % 9) as i32 * 16),
        );
        let offset = (496 - self.ticks as i32 * 80).max(0) as f32;
        self.canvas
            .sprite(Sprite::new(&images[1], -3.0 + offset, 64.0));
        if offset > 0.0 {
            return;
        }
        self.canvas
            .sprite(Sprite::new(&images[3 + self.mission - 1], -190.0, 65.0));
        self.canvas.label(
            Sprite::new(&images[13 + self.mission - 1], 80.0, 46.0),
            labels::mission(self.language, self.mission),
            Color32::from_rgb(57, 38, 17),
        );
        let stage = stage_id(campaign, self.mission);
        for (i, record) in campaign.progress.records[campaign.options.character as usize][stage]
            .iter()
            .enumerate()
        {
            let id = if record.played() {
                27 - record.score(stage).grade as usize
            } else {
                29
            };
            let sprite = Sprite::new(
                &images[id],
                -42.0 + i as f32 * 120.0,
                if id == 29 { 75.0 } else { 77.0 },
            )
            .blend(if i == self.layout {
                Blend::Opaque
            } else {
                Blend::Quarter
            });
            if id == 29 {
                self.canvas.sprite(sprite);
            } else {
                let grade = 27 - id;
                let color = labels::rank_color(grade);
                self.canvas.label(
                    sprite,
                    labels::rank(self.language, grade),
                    if i == self.layout {
                        color
                    } else {
                        color.gamma_multiply(0.4)
                    },
                );
            }
        }
    }

    fn paint_options(&mut self, campaign: &Campaign) {
        let images = &self.option_images;
        self.canvas.sprite(Sprite::new(
            &images[0],
            0.0,
            (images[0].size[1] as f32 - 240.0) / 2.0,
        ));
        if self.page == Page::Editor {
            return;
        }
        if self.page == Page::Options {
            let slide = (250 - self.ticks as i32 * 20).max(0) as f32;
            for (i, label) in ORIGINAL_OPTIONS.iter().enumerate() {
                let x = if i < 5 { -82.0 - slide } else { 82.0 + slide };
                let y = (i % 5) as f32 * 34.0 - 68.0;
                self.canvas.sprite(
                    Sprite::new(&images[7], x, y)
                        .scale(
                            if i < 5 { 1.0 } else { -1.0 },
                            if i < 5 { 1.0 } else { -1.0 },
                        )
                        .blend(if i == self.option_selection {
                            Blend::Opaque
                        } else {
                            Blend::Average
                        }),
                );
                if self.canvas.sharp_text {
                    self.canvas.label(
                        Sprite::new(&images[9], x, y + if i < 5 { 2.0 } else { -2.0 }).row(i, 10),
                        *label,
                        if i == self.option_selection {
                            Color32::from_rgb(240, 72, 54)
                        } else {
                            Color32::from_rgb(220, 207, 183)
                        },
                    );
                    continue;
                }
                self.canvas.sprite(
                    Sprite::new(&images[9], x, y + if i < 5 { 2.0 } else { -2.0 })
                        .row(i, 10)
                        .light(63)
                        .blend(Blend::Subtract),
                );
                self.canvas.sprite(
                    Sprite::new(&images[9], x, y + if i < 5 { 2.0 } else { -2.0 })
                        .row(i, 10)
                        .color(if i == self.option_selection {
                            [227, 0, 16]
                        } else {
                            [34, 0, 87]
                        })
                        .blend(Blend::Add),
                );
            }
            self.canvas.label(
                Sprite::new(&images[1], 0.0, -96.0).row(0, 6),
                "各種設定",
                Color32::WHITE,
            );
            self.canvas.label(
                Sprite::new(&images[3], 0.0, 96.0),
                "○ 決定　× 終了",
                Color32::LIGHT_GRAY,
            );
            return;
        }
        let option = match self.page {
            Page::Choice(i) => i,
            Page::Records => 0,
            Page::Volume => 3,
            Page::Controls => 5,
            Page::SoundTest => 4,
            Page::Card => 7,
            _ => 0,
        };
        let (header, row, count) = if option < 5 {
            (1, option + 1, 6)
        } else {
            (2, option - 5, 5)
        };
        self.canvas.label(
            Sprite::new(
                &images[header],
                if self.page == Page::Records {
                    -90.0
                } else {
                    0.0
                },
                -96.0,
            )
            .row(row, count),
            ORIGINAL_OPTIONS[option],
            Color32::WHITE,
        );
        if !self.canvas.sharp_text || self.page != Page::Controls {
            self.canvas.label(
                Sprite::new(
                    &images[4],
                    0.0,
                    if self.canvas.sharp_text && self.page == Page::Records {
                        106.0
                    } else {
                        96.0
                    },
                ),
                "○ 決定　× キャンセル",
                Color32::LIGHT_GRAY,
            );
        }
        match self.page {
            Page::Choice(_) | Page::Card => {
                self.canvas.sprite(Sprite::new(&images[5], 0.0, 0.0));
                let (image, first, n, count) = match self.page {
                    Page::Card => (15, 0, 2, 2),
                    _ => match option {
                        1 => (13, 0, 3, 7),
                        2 => (13, 3, 2, 7),
                        9 => (14, 0, 4, 5),
                        _ => (13, 5, 2, 7),
                    },
                };
                for i in 0..n {
                    let y = if n == 2 {
                        i as f32 * 40.0 - 20.0
                    } else if n == 4 {
                        i as f32 * 20.0 - 30.0
                    } else {
                        i as f32 * 28.0 - 28.0
                    };
                    let label = if self.page == Page::Card {
                        ["読込", "保存"][i]
                    } else {
                        match option {
                            1 => ["易", "普", "難"][i],
                            2 => ["ステレオ", "モノラル"][i],
                            9 => ["日本語", "英語", "仏蘭西語", "伊太利亜語"][i],
                            _ => ["する", "しない"][i],
                        }
                    };
                    self.canvas.label(
                        Sprite::new(&images[image], 15.0, y)
                            .row(first + i, count)
                            .light(if i == self.choice { 128 } else { 32 })
                            .blend(Blend::Subtract),
                        label,
                        if i == self.choice {
                            Color32::from_rgb(40, 27, 22)
                        } else {
                            Color32::from_rgb(119, 100, 78)
                        },
                    );
                    if i == self.choice {
                        self.canvas.sprite(
                            Sprite::new(&images[6], -38.0, y)
                                .light(255)
                                .blend(Blend::Subtract),
                        );
                        self.canvas.sprite(
                            Sprite::new(&images[6], -38.0, y)
                                .color([143, 8, 10])
                                .blend(Blend::Add),
                        );
                    }
                }
            }
            Page::Volume | Page::SoundTest => {
                self.canvas.sprite(Sprite::new(&images[18], 0.0, 0.0));
                for i in 0..2 {
                    let y = i as f32 * 48.0 - 24.0;
                    let image = if self.page == Page::Volume { 16 } else { 17 };
                    self.canvas.label(
                        Sprite::new(&images[image], -25.0, y)
                            .row(i, 2)
                            .light(if i == self.selection { 128 } else { 32 })
                            .blend(Blend::Subtract),
                        if i == 0 {
                            "音楽"
                        } else if self.page == Page::Volume {
                            "効果音"
                        } else {
                            "効果音・音声"
                        },
                        if i == self.selection {
                            Color32::from_rgb(40, 27, 22)
                        } else {
                            Color32::from_rgb(119, 100, 78)
                        },
                    );
                    if self.page == Page::Volume {
                        self.canvas.sprite(Sprite::new(&images[19], 50.0, y));
                        let value = if i == 0 {
                            self.preferences.music
                        } else {
                            self.preferences.effects
                        };
                        self.canvas.sprite(Sprite::new(
                            &images[20],
                            10.0 + f32::from(value) * 80.0 / 127.0,
                            y,
                        ));
                    } else {
                        let n = self.sound_choices[i];
                        for (j, digit) in [n / 10, n % 10].into_iter().enumerate() {
                            self.canvas.label(
                                Sprite::new(&images[21], 65.0 + j as f32 * 16.0, y).region([
                                    digit * 14,
                                    0,
                                    14,
                                    16,
                                ]),
                                digit.to_string(),
                                Color32::WHITE,
                            );
                        }
                    }
                }
            }
            Page::Controls => {
                self.canvas.sprite(Sprite::new(&images[10], 0.0, 0.0));
                self.canvas.label(
                    Sprite::new(&images[12], 0.0, -62.0)
                        .row(self.choice, 4)
                        .blend(Blend::Subtract),
                    ["通常", "その壱", "その弐", "その参"][self.choice],
                    Color32::from_rgb(40, 27, 22),
                );
                for i in 0..8 {
                    let [x, y] = self.tables.control_positions[i];
                    self.canvas.label(
                        Sprite::new(&images[11], f32::from(x), f32::from(y))
                            .row(self.tables.control_actions[self.choice][i], 6)
                            .blend(Blend::Subtract),
                        ["忍具選択", "視点変更", "忍具使用", "隠れる", "跳ぶ", "斬る"]
                            [self.tables.control_actions[self.choice][i]],
                        Color32::from_rgb(40, 27, 22),
                    );
                }
            }
            Page::Records => self.paint_records(campaign),
            _ => (),
        }
    }

    fn paint_records(&mut self, campaign: &Campaign) {
        let images = &self.option_images;
        for i in 0..2 {
            self.canvas.sprite(
                Sprite::new(&images[23 + i], 40.0 + i as f32 * 80.0, -96.0).blend(
                    if i == self.selection {
                        Blend::Opaque
                    } else {
                        Blend::Quarter
                    },
                ),
            );
        }
        let mut row = 0;
        for mission in (1..=10).chain([0]) {
            let stage = stage_id(campaign, mission);
            let records = &campaign.progress.records[self.selection][stage];
            if !records.iter().any(|r| r.played()) {
                continue;
            }
            let y = -65.0 + row as f32 * 15.0;
            self.canvas.label(
                Sprite::new(&images[26], -132.0, y)
                    .row(if mission == 0 { 10 } else { mission - 1 }, 11)
                    .blend(Blend::Add),
                [
                    "訓練",
                    "一乃巻",
                    "二乃巻",
                    "三乃巻",
                    "四乃巻",
                    "五乃巻",
                    "六乃巻",
                    "七乃巻",
                    "八乃巻",
                    "九乃巻",
                    "十乃巻",
                ][mission],
                Color32::from_rgb(220, 144, 204),
            );
            for (i, record) in records.iter().enumerate() {
                if record.played() {
                    self.canvas.sprite(
                        Sprite::new(&images[28], i as f32 * 67.0 - 77.0, y + 1.0).blend(Blend::Add),
                    );
                    let grade = record.score(stage).grade as usize;
                    self.canvas.label(
                        Sprite::new(&images[25], i as f32 * 67.0 - 77.0, y).row(4 - grade, 5),
                        labels::rank(Language::Japanese, grade),
                        labels::rank_color(grade),
                    );
                }
            }
            let score = records
                .iter()
                .filter(|r| r.played())
                .map(|r| r.score(stage).total)
                .max()
                .unwrap_or(0);
            let time = records
                .iter()
                .filter(|r| r.played())
                .map(|r| r.clock.max(0) / 30)
                .min()
                .unwrap_or(0);
            digits(
                &mut self.canvas,
                &images[22],
                score as i32,
                116.0,
                y + 2.0,
                6.0,
                true,
            );
            digits(
                &mut self.canvas,
                &images[22],
                time / 60,
                133.0,
                y + 2.0,
                6.0,
                true,
            );
            digits(
                &mut self.canvas,
                &images[22],
                time % 60,
                151.0,
                y + 2.0,
                6.0,
                true,
            );
            self.canvas.label(
                Sprite::new(&images[29], 139.0, y + 2.0).blend(Blend::Add),
                ":",
                Color32::WHITE,
            );
            row += 1;
        }
        if row > 0 {
            if self.canvas.sharp_text {
                for (x, width, label) in [
                    (-77.0, 60, "配置一"),
                    (-10.0, 60, "配置二"),
                    (57.0, 60, "配置三"),
                    (116.0, 12, "点"),
                    (142.0, 16, "時"),
                ] {
                    self.canvas.label(
                        Sprite::new(&images[27], x, -77.0).region([0, 0, width, 12]),
                        label,
                        Color32::from_rgb(104, 202, 104),
                    );
                }
            } else {
                self.canvas
                    .sprite(Sprite::new(&images[27], 25.0, -77.0).blend(Blend::Add));
            }
        }
    }

    fn paint_results(&mut self, campaign: &Campaign) {
        let crate::campaign::Screen::Results(results) = &campaign.screen else {
            return;
        };
        self.canvas
            .sprite(Sprite::new(&self.result_background, 0.0, 0.0));
        let record = results.record;
        let score = results.score;
        let best = results.best.score(campaign.game.mission.stage.id);
        let rows = [
            (
                record.criticals as i32,
                record.enemies.wrapping_sub(record.bosses) as i32,
                score.criticals,
                best.criticals,
                -53.0,
            ),
            (
                record.murders as i32,
                record.enemies as i32,
                score.murders,
                best.murders,
                -26.0,
            ),
            (record.found as i32, 0, score.stealth, best.stealth, 1.0),
            (record.friends as i32, 0, score.friends, best.friends, 27.0),
        ];
        for (a, b, current, best, y) in rows {
            digits(
                &mut self.canvas,
                &self.result_numbers,
                a,
                if y < 0.0 { 10.0 } else { 28.0 },
                y,
                12.0,
                false,
            );
            if y < 0.0 {
                digits(
                    &mut self.canvas,
                    &self.result_numbers,
                    b,
                    40.0,
                    y,
                    12.0,
                    false,
                );
            }
            digits(
                &mut self.canvas,
                &self.result_numbers,
                i32::from(current),
                82.0,
                y,
                12.0,
                false,
            );
            digits(
                &mut self.canvas,
                &self.result_numbers,
                i32::from(best),
                127.0,
                y,
                12.0,
                false,
            );
        }
        digits(
            &mut self.canvas,
            &self.result_numbers,
            i32::from(score.total),
            82.0,
            57.0,
            12.0,
            false,
        );
        digits(
            &mut self.canvas,
            &self.result_numbers,
            i32::from(best.total),
            127.0,
            57.0,
            12.0,
            false,
        );
        let time = record.clock.max(0) / 30;
        for (value, x) in [(time / 60, 65.0), (time % 60 / 10, 85.0), (time % 10, 97.0)] {
            digits(
                &mut self.canvas,
                &self.result_numbers,
                value,
                x,
                -93.0,
                12.0,
                false,
            );
        }
        let rank = &self.result_ranks[score.grade as usize];
        let brightness = (self.ticks as f32 * std::f32::consts::TAU / 90.0).sin() * 0.5 + 0.5;
        self.canvas.label(
            Sprite::new(
                rank,
                -29.0 + rank.size[0] as f32 / 2.0,
                78.0 + rank.size[1] as f32 / 2.0,
            )
            .blend(Blend::Add)
            .light(((self.ticks as f32 * std::f32::consts::TAU / 90.0).sin() * 127.0) as i32 + 127),
            labels::rank(self.language, score.grade as usize),
            labels::rank_color(score.grade as usize).gamma_multiply(brightness),
        );
        if score.grade == 4 {
            let mission = &campaign.game.mission;
            let icon = Image::from_vram(&mission.vram, mission.images[38]);
            self.canvas.sprite(
                Sprite::new(
                    &icon,
                    -128.0 + icon.size[0] as f32 / 2.0,
                    49.0 + icon.size[1] as f32 / 2.0,
                )
                .light(
                    ((self.ticks as f32 * std::f32::consts::TAU / 90.0).cos() * 80.0) as i32 + 127,
                ),
            );
        }
    }
}

fn digits(
    canvas: &mut Canvas,
    image: &Image,
    value: i32,
    x: f32,
    y: f32,
    spacing: f32,
    vertical: bool,
) {
    if canvas.sharp_text {
        let text = value.to_string();
        let width = spacing * text.len() as f32;
        canvas.text.push(crate::fonts::Text {
            position: egui::pos2(
                x - (width - spacing) / 2.0 + if vertical { 0.0 } else { 6.0 },
                y - if vertical { 5.0 } else { 3.0 },
            ),
            text,
            color: Color32::WHITE,
            size: if vertical { 8.0 } else { 12.0 },
            width,
            centered: true,
        });
        return;
    }
    for (i, digit) in value.to_string().bytes().rev().enumerate() {
        let n = if digit == b'-' {
            10
        } else {
            usize::from(digit - b'0')
        };
        if vertical && n == 10 {
            continue;
        }
        let sprite = Sprite::new(
            image,
            x - i as f32 * spacing + if vertical { 0.0 } else { 6.0 },
            y + if vertical {
                0.0
            } else {
                image.size[1] as f32 / 2.0
            },
        );
        canvas.sprite(
            if vertical {
                sprite.row(n, 10)
            } else {
                sprite.region([n * 12, 0, 12, image.size[1]])
            }
            .blend(Blend::Add),
        );
    }
}
