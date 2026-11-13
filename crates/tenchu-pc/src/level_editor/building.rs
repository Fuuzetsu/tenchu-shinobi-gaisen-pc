use super::*;
use serde::{Deserialize, Serialize};
use tenchu_assets::trial::Cell;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    pub name: String,
    pub theme: usize,
    pub width: i16,
    pub height: i16,
    pub cells: Vec<Cell>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub name: String,
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
}

impl View {
    pub fn new(name: String, camera: Camera) -> Self {
        Self {
            name,
            position: camera.position.to_array(),
            yaw: camera.yaw,
            pitch: camera.pitch,
            fov: camera.fov,
        }
    }

    pub fn camera(&self) -> Camera {
        Camera {
            position: self.position.into(),
            yaw: self.yaw,
            pitch: self.pitch,
            fov: self.fov,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.len() > 512
            || self
                .position
                .iter()
                .any(|v| !v.is_finite() || v.abs() > 320_000.0)
            || !self.yaw.is_finite()
            || !(-1.56..=1.56).contains(&self.pitch)
            || !(10.0..=120.0).contains(&self.fov)
        {
            return Err("Invalid saved camera view".into());
        }
        Ok(())
    }
}

pub fn bounds(a: Point, b: Point) -> (Point, Point) {
    (
        Point {
            x: a.x.min(b.x),
            z: a.z.min(b.z),
        },
        Point {
            x: a.x.max(b.x),
            z: a.z.max(b.z),
        },
    )
}

impl Stamp {
    pub fn clipboard_text(&self) -> Result<String> {
        self.validate()?;
        Ok(format!(
            "tenchu-stamp-v1\n{}",
            serde_json::to_string(self).map_err(|e| e.to_string())?
        ))
    }

    pub fn from_clipboard(text: &str) -> Result<Self> {
        if text.len() > 128 * 1024 {
            return Err("Terrain clipboard is too large".into());
        }
        let json = text
            .strip_prefix("tenchu-stamp-v1\n")
            .ok_or("Clipboard does not contain a terrain stamp")?;
        let stamp: Self =
            serde_json::from_str(json).map_err(|e| format!("Read terrain stamp: {e}"))?;
        stamp.validate()?;
        Ok(stamp)
    }

    pub fn copy(mission: &Mission, a: Point, b: Point) -> Result<Self> {
        if !a.valid() || !b.valid() {
            return Err("Select an area inside the map".into());
        }
        let (a, b) = bounds(a, b);
        let cells = (a.z..=b.z)
            .flat_map(|z| (a.x..=b.x).map(move |x| mission.map[z as usize * 32 + x as usize]))
            .collect();
        Ok(Self {
            name: "Terrain stamp".into(),
            theme: mission.theme,
            width: b.x - a.x + 1,
            height: b.z - a.z + 1,
            cells,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.len() > 512
            || self.theme >= 12
            || !(1..=32).contains(&self.width)
            || !(1..=32).contains(&self.height)
            || self.cells.len() != self.width as usize * self.height as usize
        {
            return Err("Invalid terrain stamp dimensions or theme".into());
        }
        Ok(())
    }

    pub fn rotate(&mut self) {
        let mut cells = self.cells.clone();
        for z in 0..self.height {
            for x in 0..self.width {
                let mut cell = self.cells[(z * self.width + x) as usize];
                for tile in [&mut cell.ground, &mut cell.ornament] {
                    if *tile != 255 {
                        *tile = (*tile & 63) | (((*tile >> 6) + 1) % 4) << 6;
                    }
                }
                cells[(x * self.height + self.height - 1 - z) as usize] = cell;
            }
        }
        self.cells = cells;
        std::mem::swap(&mut self.width, &mut self.height);
    }

    pub fn paste(&self, mission: &mut Mission, theme: &Theme, at: Point) -> Result<()> {
        self.validate()?;
        if self.theme != mission.theme {
            return Err("This stamp uses another terrain theme".into());
        }
        if !at.valid() || at.x + self.width > 32 || at.z + self.height > 32 {
            return Err("The whole stamp must fit inside the map".into());
        }
        let mut next = mission.clone();
        // Free the destination ornaments before checking the source's global limit.
        for z in 0..self.height {
            for x in 0..self.width {
                next.map[(at.z + z) as usize * 32 + (at.x + x) as usize].ornament = 255;
            }
        }
        for (i, cell) in self.cells.iter().enumerate() {
            let point = Point {
                x: at.x + i as i16 % self.width,
                z: at.z + i as i16 / self.width,
            };
            editing::paint(&mut next, theme, point, false, cell.ground)?;
            if cell.ornament != 255 {
                editing::paint(&mut next, theme, point, true, cell.ornament)?;
            }
        }
        theme.validate(&next)?;
        *mission = next;
        Ok(())
    }
}

impl Editor {
    pub(super) fn copy_area(&mut self, ctx: &egui::Context) {
        let result = self
            .selection
            .ok_or_else(|| "Drag an area first".into())
            .and_then(|(a, b)| Stamp::copy(&self.project.mission, a, b));
        match result {
            Ok(stamp) => {
                self.notice = format!(
                    "Copied {} × {} cells · Ctrl+V to place · R rotates",
                    stamp.width, stamp.height
                );
                if let Ok(text) = stamp.clipboard_text() {
                    ctx.copy_text(text);
                }
                self.clipboard = Some(stamp);
            }
            Err(error) => self.notice = error,
        }
    }

    pub(super) fn fill_area(&mut self) -> Result<()> {
        let (a, b) = self.selection.ok_or("Drag an area first")?;
        let (a, b) = bounds(a, b);
        let mut next = self.project.mission.clone();
        let tile = self.brush | self.rotation << 6;
        for z in a.z..=b.z {
            for x in a.x..=b.x {
                editing::paint(
                    &mut next,
                    self.theme.as_ref().unwrap(),
                    Point { x, z },
                    !self.ground,
                    tile,
                )?;
            }
        }
        self.history.begin(&self.project);
        self.project.mission = next;
        self.commit();
        Ok(())
    }

    pub(super) fn sample_cell(&mut self) {
        let cell = self.project.mission.map[self.cursor.z as usize * 32 + self.cursor.x as usize];
        let tile = if self.ground {
            cell.ground
        } else {
            cell.ornament
        };
        if tile == 255 {
            self.tool = Tool::Erase;
        } else {
            self.brush = tile & 63;
            self.rotation = tile >> 6;
            self.tool = Tool::Paint;
        }
        self.panel = Panel::Terrain;
    }

    pub(super) fn region_from_area(&mut self) -> Result<()> {
        let (a, b) = self.selection.ok_or("Drag an area first")?;
        if self.project.events.len() >= 256 {
            return Err("A project supports 256 events".into());
        }
        let (a, b) = bounds(a, b);
        let height = i32::from(self.theme.as_ref().unwrap().height(
            &self.project.mission,
            a.x,
            a.z,
            false,
        )) * 1000;
        self.history.begin(&self.project);
        self.event = self.project.events.len();
        self.project.events.push(project::Event {
            name: "Selected region".into(),
            trigger: project::Trigger::Enter {
                min: [
                    i32::from(a.x) * 2000,
                    height - 8000,
                    -i32::from(b.z + 1) * 2000,
                ],
                max: [
                    i32::from(b.x + 1) * 2000,
                    height + 2000,
                    -i32::from(a.z) * 2000,
                ],
            },
            ..Default::default()
        });
        self.commit();
        self.panel = Panel::Events;
        Ok(())
    }

    pub(super) fn area_controls(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Areas & reusable stamps").default_open(self.ephemeral).show(ui, |ui| {
            if ui.selectable_label(self.tool == Tool::Area, "Select rectangular area").clicked() { self.tool = Tool::Area; }
            if let Some((a, b)) = self.selection {
                let (a, b) = bounds(a, b);
                ui.label(format!("{}, {} → {}, {} · {} cells", a.x, a.z, b.x, b.z, (b.x-a.x+1)*(b.z-a.z+1)));
                ui.horizontal(|ui| {
                    if ui.button("Copy  Ctrl+C").clicked() { self.copy_area(ui.ctx()); }
                    if ui.button("Fill with brush").clicked() && let Err(e) = self.fill_area() { self.notice = e; }
                });
                if ui.button("Create region event").clicked() && let Err(e) = self.region_from_area() { self.notice = e; }
            }
            if let Some(stamp) = &mut self.clipboard {
                ui.text_edit_singleline(&mut stamp.name);
                ui.label(format!("{} × {} · {}", stamp.width, stamp.height, tenchu_assets::trial::THEMES[stamp.theme]));
                ui.horizontal(|ui| {
                    if ui.button("Place  Ctrl+V").clicked() { self.tool = Tool::Stamp; }
                    if ui.button("Rotate  R").clicked() { stamp.rotate(); }
                    if ui.add_enabled(self.project.stamps.len() < 32, egui::Button::new("Keep stamp")).clicked() { self.project.stamps.push(stamp.clone()); }
                });
            }
            let mut remove = None;
            for (i, stamp) in self.project.stamps.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.button(format!("{} · {}×{}", stamp.name, stamp.width, stamp.height)).clicked() {
                        self.clipboard = Some(stamp.clone()); self.tool = Tool::Stamp;
                    }
                    if ui.small_button("×").on_hover_text("Remove saved stamp").clicked() { remove = Some(i); }
                });
            }
            if let Some(index) = remove { self.project.stamps.remove(index); }
            ui.small("Stamps include both terrain layers, keep source placement rules, and are saved with the project. A blocked paste leaves the map unchanged.");
        });
    }

    pub(super) fn camera_views(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Camera views", |ui| {
            if ui
                .add_enabled(
                    self.unlocked && self.project.views.len() < 16,
                    egui::Button::new("Save current view"),
                )
                .clicked()
            {
                self.history.begin(&self.project);
                self.project.views.push(View::new(
                    format!("View {}", self.project.views.len() + 1),
                    self.camera,
                ));
                self.commit();
            }
            self.history.begin(&self.project);
            let mut remove = None;
            for (i, view) in self.project.views.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui.button(format!("Go {}", i + 1)).clicked() {
                        self.camera = view.camera();
                        self.grid = false;
                    }
                    ui.add_enabled_ui(self.unlocked, |ui| {
                        ui.add(egui::TextEdit::singleline(&mut view.name).desired_width(130.0));
                        if ui.small_button("×").clicked() {
                            remove = Some(i);
                        }
                    });
                });
            }
            if let Some(index) = remove {
                self.project.views.remove(index);
            }
            if self.project.views.is_empty() {
                ui.label("Save angles for building and screenshots.");
            }
            ui.small("Camera views travel with your project; they do not change playtest cameras.");
        });
    }

    pub(super) fn move_scenery(&mut self, target: [f32; 3]) {
        if !self.unlocked {
            return;
        }
        if let Some(prop) = self
            .scenery_selection
            .and_then(|i| self.project.scenery.get_mut(i))
        {
            // Floor contact is independent of horizontal grid snapping.
            let position = std::array::from_fn(|axis| {
                let value = if axis != 1 && self.snap != 0 {
                    (target[axis] / self.snap as f32).round() * self.snap as f32
                } else {
                    target[axis]
                };
                (value.round() as i32).clamp(-320_000, 320_000)
            });
            if prop.position != position {
                self.history.begin(&self.project);
                self.project.scenery[self.scenery_selection.unwrap()].position = position;
            }
        }
    }

    fn scenery_position(&mut self) -> Option<&mut [i32; 3]> {
        if let Some(i) = self.campaign_prop {
            return self
                .project
                .campaign
                .as_mut()?
                .props
                .get_mut(i)
                .map(|p| &mut p.position);
        }
        self.project
            .scenery
            .get_mut(self.scenery_selection?)
            .map(|p| &mut p.position)
    }

    pub(super) fn nudge_scenery(&mut self, axis: usize, direction: i32) {
        let step = self.snap.max(100);
        self.history.begin(&self.project);
        if let Some(position) = self.scenery_position() {
            position[axis] = (position[axis] + direction * step).clamp(-320_000, 320_000);
        }
        self.commit();
    }

    pub(super) fn ground_scenery(&mut self) {
        let Some(p) = self.scenery_position().copied() else {
            return;
        };
        let top = self
            .surfaces
            .iter()
            .flat_map(|s| s.corners)
            .map(|v| v.y)
            .reduce(f32::min)
            .unwrap_or(0.0)
            - 1000.0;
        let origin = glam::vec3(p[0] as f32, top, p[2] as f32);
        if let Some(hit) = crate::debug::terrain::pick(&self.surfaces, origin, glam::Vec3::Y) {
            self.history.begin(&self.project);
            self.scenery_position().unwrap()[1] = hit.y;
            self.commit();
        } else {
            self.notice = "No solid terrain below this scenery".into();
        }
    }
}
