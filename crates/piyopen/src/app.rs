mod central_panel;
mod piano_roll;
mod top_panel;

use {
    crate::{add_fallback_font_to_egui, config::Config},
    eframe::egui::{self, mutex::Mutex},
    egui_file_dialog::FileDialog,
    piyopiyo::{Event, N_KEYS, Track},
    std::{panic::AssertUnwindSafe, path::Path, sync::Arc},
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
            paused: true,
            volume: 1.0,
        })
    }
}

pub struct PiyopenApp {
    shared: Option<Arc<Mutex<SharedPiyoState>>>,
    file_dia: FileDialog,
    _audio: Option<tinyaudio::OutputDevice>,
    track_select: TrackSelect,
    open_path: Option<String>,
    popup_msg: Option<String>,
    waveform_last_pos: Option<egui::Pos2>,
    envelope_last_pos: Option<egui::Pos2>,
    cfg: Config,
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
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            if shared.paused {
                for samp in buf.as_chunks_mut().0 {
                    *samp = shared.player.next_sample();
                }
            } else {
                shared.player.render_next(&mut buf);
            }
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
    pub fn new(path: Option<String>, mut cfg: Config) -> anyhow::Result<Self> {
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
                cfg.last_opened = Some(path.clone());
                (Some(path), Some(shared), Some(audio))
            }
            None => (None, None, None),
        };
        Ok(Self {
            shared,
            file_dia: FileDialog::new().canonicalize_paths(false),
            _audio: audio,
            track_select: TrackSelect::Melody(0),
            open_path,
            popup_msg,
            waveform_last_pos: None,
            envelope_last_pos: None,
            cfg,
        })
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
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
        let piano_keys: [bool; N_KEYS as usize] = ctx.input(|inp| {
            let mut down = [false; _];
            let kb_keys = [
                // Lower
                &[egui::Key::Z][..],
                &[egui::Key::S],
                &[egui::Key::X],
                &[egui::Key::D],
                &[egui::Key::C],
                &[egui::Key::V],
                &[egui::Key::G],
                &[egui::Key::B],
                &[egui::Key::H, egui::Key::Num1],
                &[egui::Key::N, egui::Key::Q],
                &[egui::Key::J, egui::Key::Num2],
                &[egui::Key::M, egui::Key::W],
                // Upper
                &[egui::Key::E, egui::Key::Comma],
                &[egui::Key::Num4, egui::Key::L],
                &[egui::Key::R, egui::Key::Period],
                &[egui::Key::Num5, egui::Key::Semicolon],
                &[egui::Key::T, egui::Key::Slash],
                &[egui::Key::Y],
                &[egui::Key::Num7],
                &[egui::Key::U],
                &[egui::Key::Num8],
                &[egui::Key::I],
                &[egui::Key::Num9],
                &[egui::Key::O],
            ];
            for ev in &inp.events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } = ev
                    && let Some(idx) = kb_keys
                        .iter()
                        .position(|kb_keys| kb_keys.iter().any(|kb_key| key == kb_key))
                {
                    down[idx] = true;
                }
            }
            down
        });
        if let Some(shared) = &mut self.shared {
            let event = Event::from_keydown_array(piano_keys);
            let mut shared = shared.lock();
            match self.track_select {
                TrackSelect::Melody(idx) => {
                    shared.player.song.melody_tracks[idx as usize].do_event(event)
                }
                TrackSelect::Percussion => shared.player.song.percussion_track.do_event(event),
            }
        }
        if ctrl && key_o {
            if let Some(path) = &self.open_path
                && let Some(parent) = <_ as AsRef<Path>>::as_ref(path).parent()
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
                        let path_as_string = path.as_os_str().to_str().unwrap().to_owned();
                        self.open_path = Some(path_as_string.clone());
                        self.cfg.last_opened = Some(path_as_string);
                        self._audio.get_or_insert_with(|| {
                            spawn_playback_thread(self.shared.as_ref().unwrap().clone())
                        });
                    }
                    Err(e) => self.popup_msg = Some(e.to_string()),
                },
                FileDialogOp::AddFont => add_fallback_font_to_egui(ctx, "fallback", &path).unwrap(),
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
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let result = crate::config::save(&self.cfg);
        if let Err(e) = result {
            eprintln!("Failed to save config: {e}");
        }
    }
}
