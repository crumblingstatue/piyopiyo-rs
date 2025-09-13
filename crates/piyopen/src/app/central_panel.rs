use {
    crate::{
        app::{PiyopenApp, TrackSelect},
        draw_widgets::{envelope_widget, waveform_widget},
    },
    eframe::egui,
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
        crate::app::piano_roll::ui(ui, app.track_select, &mut shared, n_events);
    }
}
