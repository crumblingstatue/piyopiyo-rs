use {
    crate::{
        app::{PiyopenApp, TrackSelect},
        draw_widgets::{envelope_widget, waveform_widget},
    },
    eframe::egui,
    piyopiyo::{N_KEYS, PianoKey, piano_keys},
};

pub fn ui(app: &mut PiyopenApp, ui: &mut egui::Ui) {
    if let Some(shared) = app.shared.as_mut() {
        let mut shared = shared.lock();
        ui.style_mut().spacing.slider_width = ui.available_width() - 100.0;
        let n_events = (shared.player.n_events() as u32).saturating_sub(1);
        ui.horizontal(|ui| {
            ui.add(egui::Slider::new(
                &mut shared.player.event_cursor,
                0..=n_events,
            ));
            ui.label(format!("/{}", shared.player.n_events()));
        });

        ui.separator();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut app.track_select, TrackSelect::Melody(0), "1");
            ui.selectable_value(&mut app.track_select, TrackSelect::Melody(1), "2");
            ui.selectable_value(&mut app.track_select, TrackSelect::Melody(2), "3");
            ui.selectable_value(&mut app.track_select, TrackSelect::Percussion, "Drum");
            ui.separator();
            let base = match app.track_select {
                TrackSelect::Melody(idx) => {
                    let track = &mut shared.player.song.melody_tracks[usize::from(idx)];
                    ui.label("Wave");
                    waveform_widget(ui, &mut track.waveform, &mut app.waveform_last_pos);
                    ui.label("Envelope");
                    envelope_widget(ui, &mut track.envelope, &mut app.envelope_last_pos);
                    ui.label("Octave");
                    ui.add(
                        egui::DragValue::new(&mut track.octave)
                            .range(0..=7)
                            .speed(0.05),
                    );
                    ui.label("Length");
                    ui.add(egui::DragValue::new(&mut track.len).speed(100.0));
                    &mut track.base
                }
                TrackSelect::Percussion => {
                    let track = &mut shared.player.song.percussion_track;
                    &mut track.base
                }
            };
            ui.label("Volume");
            ui.add(
                egui::DragValue::new(&mut base.vol)
                    .range(0..=300)
                    .speed(1.0),
            );
            ui.separator();
            ui.label("Wait")
                .on_hover_text("How much to wait before next event (in milliseconds)");
            ui.add(egui::DragValue::new(&mut shared.player.song.event_wait_ms).range(1..=5000));
            ui.label("Repeat");
            ui.add(egui::DragValue::new(
                &mut shared.player.song.repeat_range.start,
            ));
            ui.add(egui::DragValue::new(
                &mut shared.player.song.repeat_range.end,
            ));
        });
        ui.separator();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            let node_size = 8.0;
            let node_gapped = 16.0;
            let scrollbar_gap = node_gapped;
            let content_size = egui::vec2(
                n_events as f32 * node_gapped,
                (N_KEYS as f32 * node_gapped) + scrollbar_gap,
            );
            let (rect, re) = ui.allocate_exact_size(content_size, egui::Sense::click_and_drag());
            let mut x = rect.min.x;
            let y_off = rect.min.y;
            let p = ui.painter();
            let event_cursor = shared.player.event_cursor;
            let paused = shared.paused;
            let events = match app.track_select {
                TrackSelect::Melody(n) => {
                    &mut shared.player.song.melody_tracks[usize::from(n)].base.events
                }
                TrackSelect::Percussion => &mut shared.player.song.percussion_track.base.events,
            };
            for event in &mut *events {
                let mut y = y_off + (N_KEYS as f32 * node_gapped);
                for key in piano_keys() {
                    if event.key_down(key) {
                        p.rect_filled(
                            egui::Rect::from_center_size(
                                egui::pos2(x, y),
                                egui::vec2(node_size, node_size),
                            ),
                            1.0,
                            egui::Color32::WHITE,
                        );
                    }
                    y -= node_gapped;
                }
                x += node_gapped;
            }
            let cx = (event_cursor as f32 * node_gapped) + rect.min.x;
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
                    let event = pos / node_gapped;
                    let event_off = event.x as usize;
                    let key = event.y as PianoKey;
                    match action {
                        Action::Add => events[event_off].set_key_down(N_KEYS - key),
                        Action::Del => events[event_off].unset_key_down(N_KEYS - key),
                        Action::SetPos => shared.player.event_cursor = event_off as u32,
                    }
                }
            }
        });
        ui.separator();
    }
}
