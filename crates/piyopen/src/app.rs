mod central_panel;
mod top_panel;

use {
    eframe::{
        egui::{self, mutex::Mutex},
        epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    },
    egui_file_dialog::FileDialog,
    std::{
        ffi::OsString,
        panic::AssertUnwindSafe,
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
            player: piyopiyo::Player::new(&data, SAMPLE_RATE)?,
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
    waveform_last_pos: Option<egui::Pos2>,
    envelope_last_pos: Option<egui::Pos2>,
}

const SAMPLE_RATE: u32 = 48_000;
const N_BUFFERED_SAMPLES: usize = 512;

fn spawn_playback_thread(shared: Arc<Mutex<SharedPiyoState>>) -> tinyaudio::OutputDevice {
    let params = tinyaudio::OutputDeviceParameters {
        sample_rate: SAMPLE_RATE as usize,
        channels_count: 2,
        channel_sample_count: N_BUFFERED_SAMPLES,
    };
    tinyaudio::run_output_device(params, move |data| {
        let mut buf: [i16; N_BUFFERED_SAMPLES * 2] = [0; _];
        let mut shared = shared.lock();
        if shared.paused {
            data.fill(0.);
            return;
        }
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            shared.player.render_next(&mut buf);
        }));
        if let Err(e) = result {
            eprintln!("piyopiyo panic: {e:?}");
        }
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
            waveform_last_pos: None,
            envelope_last_pos: None,
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
            top_panel::ui(self, ui, key_space);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            central_panel::ui(self, ui);
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
