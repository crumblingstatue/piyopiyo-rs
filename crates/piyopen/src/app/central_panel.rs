use {
    crate::{
        app::{PiyopenApp, SharedPiyoState, TrackSelect},
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
        crate::app::piano_roll::ui(ui, app.track_select, &mut shared, n_events);
        ui.separator();
        track_selector_ui(
            ui,
            &mut app.track_select,
            &mut shared,
            &mut app.waveform_last_pos,
            &mut app.envelope_last_pos,
        );
    }
}

fn track_selector_ui(
    ui: &mut egui::Ui,
    track_select: &mut TrackSelect,
    shared: &mut SharedPiyoState,
    waveform_last_pos: &mut Option<egui::Pos2>,
    envelope_last_pos: &mut Option<egui::Pos2>,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.selectable_value(track_select, TrackSelect::Melody(0), "🎵 Track 1");
            ui.selectable_value(track_select, TrackSelect::Melody(1), "🎵 Track 2");
            ui.selectable_value(track_select, TrackSelect::Melody(2), "🎵 Track 3");
            ui.selectable_value(track_select, TrackSelect::Percussion, "🔩 Track P");
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.label("Wait")
                    .on_hover_text("How much to wait before next event (in milliseconds)");
                ui.add(egui::DragValue::new(&mut shared.player.song.event_wait_ms).range(1..=5000));
            });

            ui.horizontal(|ui| {
                ui.label("Repeat");
                ui.add(egui::DragValue::new(
                    &mut shared.player.song.repeat_range.start,
                ));
                ui.add(egui::DragValue::new(
                    &mut shared.player.song.repeat_range.end,
                ));
            });
        });
        match *track_select {
            TrackSelect::Melody(idx) => {
                let track = &mut shared.player.song.melody_tracks[usize::from(idx)];
                waveform_widget(ui, &mut track.waveform, waveform_last_pos);
                ui.vertical(|ui| {
                    envelope_widget(ui, &mut track.envelope, envelope_last_pos);
                    ui.label("Octave");
                    ui.add(
                        egui::DragValue::new(&mut track.octave)
                            .range(0..=7)
                            .speed(0.05),
                    );
                    ui.label("Length");
                    ui.add(egui::DragValue::new(&mut track.len).speed(100.0));
                    ui.label("Volume");
                    ui.add(
                        egui::DragValue::new(&mut track.base.vol)
                            .range(0..=300)
                            .speed(1.0),
                    );
                });
            }
            TrackSelect::Percussion => {
                let track = &mut shared.player.song.percussion_track;
                ui.label("Volume");
                ui.add(
                    egui::DragValue::new(&mut track.base.vol)
                        .range(0..=300)
                        .speed(1.0),
                );
            }
        };
    });
}
