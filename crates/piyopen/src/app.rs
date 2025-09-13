use {
    eframe::{
        egui::{self, mutex::Mutex},
        epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    },
    egui_file_dialog::FileDialog,
    piyopiyo::{N_KEYS, PianoKey, piano_keys},
    std::{
        ffi::OsString,
        path::{Path, PathBuf},
        sync::Arc,
    },
};

struct SharedPiyoState {
    player: piyopiyo::Player,
    paused: bool,
    // 0..=1 range
    volume: f32,
}

impl SharedPiyoState {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        Ok(SharedPiyoState {
            player: piyopiyo::Player::new(&data)?,
            paused: false,
            volume: 1.0,
        })
    }
}

pub struct PiyopenApp {
    shared: Option<Arc<Mutex<SharedPiyoState>>>,
    file_dia: FileDialog,
    _audio: Option<tinyaudio::OutputDevice>,
    track_select: TrackSelect,
    open_path: Option<PathBuf>,
    popup_msg: Option<String>,
}

fn spawn_playback_thread(shared: Arc<Mutex<SharedPiyoState>>) -> tinyaudio::OutputDevice {
    let params = tinyaudio::OutputDeviceParameters {
        sample_rate: 44_100,
        channels_count: 2,
        channel_sample_count: 2048,
    };
    tinyaudio::run_output_device(params, move |data| {
        let mut buf: [i16; 4096] = [0; _];
        let mut shared = shared.lock();
        if shared.paused {
            data.fill(0.);
            return;
        }
        shared.player.render_next(&mut buf);
        for (f, i) in data.iter_mut().zip(&mut buf) {
            *f = (*i as f32 / i16::MAX as f32) * shared.volume;
        }
    })
    .unwrap()
}

impl PiyopenApp {
    pub fn new(path: Option<OsString>) -> Self {
        let mut popup_msg = None;
        let (open_path, shared, audio) = match path {
            Some(path) => 'block: {
                let shared = match SharedPiyoState::new(&path) {
                    Ok(new) => Arc::new(Mutex::new(new)),
                    Err(e) => {
                        popup_msg = Some(e.to_string());
                        break 'block (None, None, None);
                    }
                };
                let audio = spawn_playback_thread(shared.clone());
                (Some(path.into()), Some(shared), Some(audio))
            }
            None => (None, None, None),
        };
        Self {
            shared,
            file_dia: FileDialog::new().canonicalize_paths(false),
            _audio: audio,
            track_select: TrackSelect::Melody(0),
            open_path,
            popup_msg,
        }
    }
}

#[derive(PartialEq, Eq)]
enum TrackSelect {
    Melody(u8),
    Percussion,
}

enum FileDialogOp {
    OpenFile,
    AddFont,
}

impl eframe::App for PiyopenApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        let [ctrl, key_o, key_r, key_space] = ctx.input(|inp| {
            [
                inp.modifiers.ctrl,
                inp.key_pressed(egui::Key::O),
                inp.key_pressed(egui::Key::R),
                inp.key_pressed(egui::Key::Space),
            ]
        });
        if ctrl && key_o {
            if let Some(path) = &self.open_path
                && let Some(parent) = path.parent()
            {
                self.file_dia.config_mut().initial_directory = parent.to_path_buf();
            }
            self.file_dia.pick_file();
            self.file_dia.set_user_data(FileDialogOp::OpenFile);
        }
        if ctrl
            && key_r
            && let Some(shared) = &mut self.shared
            && let Some(path) = &self.open_path
        {
            match SharedPiyoState::new(path) {
                Ok(new) => *shared.lock() = new,
                Err(e) => self.popup_msg = Some(e.to_string()),
            }
        }
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        if let Some(path) = &self.open_path
                            && let Some(parent) = path.parent()
                        {
                            self.file_dia.config_mut().initial_directory = parent.to_path_buf();
                        }
                        self.file_dia.pick_file();
                        self.file_dia.set_user_data(FileDialogOp::OpenFile);
                    }
                    if let Some(shared) = &mut self.shared
                        && let Some(path) = &self.open_path
                        && ui.button("Reload").clicked()
                    {
                        match SharedPiyoState::new(path) {
                            Ok(new) => *shared.lock() = new,
                            Err(e) => self.popup_msg = Some(e.to_string()),
                        }
                    }
                    if ui.button("🗛 Add fallback font").clicked() {
                        self.file_dia.pick_file();
                        self.file_dia.set_user_data(FileDialogOp::AddFont);
                    }
                });
                if let Some(shared) = self.shared.as_mut() {
                    let mut shared = shared.lock();
                    let label = if shared.paused { "Resume" } else { "Pause" };
                    if ui.button(label).clicked() || key_space {
                        shared.paused ^= true;
                    }
                    ui.label("volume");
                    ui.add(egui::Slider::new(&mut shared.volume, 0.0..=1.0));
                }
                if let Some(path) = &self.open_path {
                    ui.label(path.display().to_string());
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(shared) = self.shared.as_mut() {
                let mut shared = shared.lock();
                ui.style_mut().spacing.slider_width = ui.available_width() - 100.0;
                let n_notes = (shared.player.n_notes() as u32).saturating_sub(1);
                ui.add(egui::Slider::new(
                    &mut shared.player.note_cursor,
                    0..=n_notes,
                ));
                ui.label(format!(
                    "{}/{}",
                    shared.player.note_cursor,
                    shared.player.n_notes()
                ));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.track_select, TrackSelect::Melody(0), "1");
                    ui.selectable_value(&mut self.track_select, TrackSelect::Melody(1), "2");
                    ui.selectable_value(&mut self.track_select, TrackSelect::Melody(2), "3");
                    ui.selectable_value(&mut self.track_select, TrackSelect::Percussion, "Drum");
                    ui.separator();
                    let base = match self.track_select {
                        TrackSelect::Melody(idx) => {
                            let track = &mut shared.player.melody_tracks[usize::from(idx)];
                            ui.label("Octave");
                            ui.add(egui::DragValue::new(&mut track.octave).range(0..=7));
                            ui.label("Length");
                            ui.add(egui::DragValue::new(&mut track.len));
                            &mut track.base
                        }
                        TrackSelect::Percussion => {
                            let track = &mut shared.player.percussion_track;
                            &mut track.base
                        }
                    };
                    ui.label("Volume");
                    ui.add(egui::DragValue::new(&mut base.vol).range(0..=300));
                });
                ui.separator();
                egui::ScrollArea::horizontal().show(ui, |ui| {
                    let node_size = 8.0;
                    let node_gapped = 16.0;
                    let scrollbar_gap = node_gapped;
                    // Virtual size of your piano roll content
                    let content_size = egui::vec2(
                        n_notes as f32 * node_gapped,
                        (N_KEYS as f32 * node_gapped) + scrollbar_gap,
                    );

                    // Allocate that much space inside the scroll area
                    let (rect, re) =
                        ui.allocate_exact_size(content_size, egui::Sense::click_and_drag());
                    let mut x = rect.min.x;
                    let y_off = rect.min.y;
                    let p = ui.painter();
                    let note_cursor = shared.player.note_cursor;
                    let paused = shared.paused;
                    let notes = match self.track_select {
                        TrackSelect::Melody(n) => {
                            &mut shared.player.melody_tracks[usize::from(n)].base.notes
                        }
                        TrackSelect::Percussion => &mut shared.player.percussion_track.base.notes,
                    };
                    for note in &mut *notes {
                        let mut y = y_off + (N_KEYS as f32 * node_gapped);
                        for key in piano_keys() {
                            if note.key_down(key) {
                                p.rect_filled(
                                    egui::Rect::from_center_size(
                                        egui::pos2(x, y),
                                        egui::vec2(node_size, node_size),
                                    ),
                                    1.0,
                                    egui::Color32::PURPLE,
                                );
                            }
                            y -= node_gapped;
                        }
                        x += node_gapped;
                    }
                    let cx = (note_cursor as f32 * node_gapped) + rect.min.x;
                    // Keep the playback cursor in view when not paused
                    if !paused && !ui.clip_rect().contains(egui::pos2(cx, rect.min.y)) {
                        let rect = egui::Rect::from_min_max(
                            egui::pos2(cx, rect.min.y),
                            egui::pos2(cx + 1.0, rect.max.y),
                        );
                        ui.scroll_to_rect(rect, Some(egui::Align::Min));
                    }
                    p.line_segment(
                        [egui::pos2(cx, rect.min.y), egui::pos2(cx, rect.max.y)],
                        egui::Stroke::new(1.0, egui::Color32::YELLOW),
                    );
                    if let Some(pos) = re.interact_pointer_pos() {
                        enum Action {
                            Add,
                            Del,
                            SetPos,
                        }
                        let action = ui.input(|inp| {
                            if inp.pointer.button_pressed(egui::PointerButton::Primary) {
                                Some(Action::Add)
                            } else if inp.pointer.button_pressed(egui::PointerButton::Secondary) {
                                Some(Action::Del)
                            } else if inp.pointer.button_pressed(egui::PointerButton::Middle) {
                                Some(Action::SetPos)
                            } else {
                                None
                            }
                        });
                        if let Some(action) = action {
                            let pos = pos - rect.min.to_vec2();
                            let note = pos / node_gapped;
                            let note_off = note.x as usize;
                            let key = note.y as PianoKey;
                            match action {
                                Action::Add => notes[note_off].set_key_down(N_KEYS - key),
                                Action::Del => notes[note_off].unset_key_down(N_KEYS - key),
                                Action::SetPos => shared.player.note_cursor = note_off as u32,
                            }
                        }
                    }
                });
                ui.separator();
            }
        });
        self.file_dia.update(ctx);
        if let Some(path) = self.file_dia.take_picked()
            && let Some(op) = self.file_dia.user_data::<FileDialogOp>()
        {
            match op {
                FileDialogOp::OpenFile => match SharedPiyoState::new(&path) {
                    Ok(new) => {
                        match &mut self.shared {
                            Some(shared) => *shared.lock() = new,
                            None => self.shared = Some(Arc::new(Mutex::new(new))),
                        }
                        self.open_path = Some(path);
                        self._audio.get_or_insert_with(|| {
                            spawn_playback_thread(self.shared.as_ref().unwrap().clone())
                        });
                    }
                    Err(e) => self.popup_msg = Some(e.to_string()),
                },
                FileDialogOp::AddFont => match std::fs::read(path) {
                    Ok(data) => {
                        let data = egui::FontData::from_owned(data);
                        ctx.add_font(FontInsert::new(
                            "fallback",
                            data,
                            vec![InsertFontFamily {
                                family: egui::FontFamily::Proportional,
                                priority: FontPriority::Lowest,
                            }],
                        ));
                    }
                    Err(e) => {
                        eprintln!("Failed to add font: {e}");
                    }
                },
            }
        }
        if let Some(msg) = &self.popup_msg {
            let mut close = false;
            egui::Modal::new("msg_popup".into()).show(ctx, |ui| {
                ui.label(msg);
                if ui.button("Ok").clicked() {
                    close = true;
                }
            });
            if close {
                self.popup_msg = None;
            }
        }
    }
}
