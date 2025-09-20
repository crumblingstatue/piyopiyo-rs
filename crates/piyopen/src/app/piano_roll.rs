use {
    crate::app::{SharedPiyoState, TrackSelect},
    eframe::egui,
    piyopiyo::{N_KEYS, PianoKey, piano_keys},
};

pub fn ui(
    ui: &mut egui::Ui,
    track_select: TrackSelect,
    shared: &mut SharedPiyoState,
    n_events: u32,
) {
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
        let events = match track_select {
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
                    Action::Del => events[event_off].set_key_up(N_KEYS - key),
                    Action::SetPos => shared.player.event_cursor = event_off as u32,
                }
            }
        }
    });
}
