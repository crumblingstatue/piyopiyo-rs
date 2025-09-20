use {
    crate::app::{SharedPiyoState, TrackSelect},
    eframe::egui,
    piyopiyo::{Event, N_KEYS, PianoKey, Track, piano_keys},
};

pub fn ui(
    ui: &mut egui::Ui,
    track_select: TrackSelect,
    shared: &mut SharedPiyoState,
    n_events: u32,
) {
    let track = track_sel_dyn(track_select, shared);
    let timers = track.timers();
    let max_time = track.note_duration(0);
    let mut key_clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        key_clicked = piano_keys_ui(ui, &timers, max_time, egui::vec2(96.0, 16.0));
        piano_hscroll_ui(ui, track_select, shared, n_events);
    });
    if let Some(key) = key_clicked {
        play_key(track_select, shared, key);
    }
}

fn play_key(track_select: TrackSelect, shared: &mut SharedPiyoState, key: u8) {
    let mut down = [false; _];
    down[key as usize] = true;
    let track = track_sel_dyn(track_select, shared);
    track.do_event(Event::from_keydown_array(down));
}

fn track_sel_dyn(track_select: TrackSelect, shared: &mut SharedPiyoState) -> &mut dyn Track {
    match track_select {
        TrackSelect::Melody(idx) => &mut shared.player.song.melody_tracks[idx as usize],
        TrackSelect::Percussion => &mut shared.player.song.percussion_track,
    }
}

fn piano_hscroll_ui(
    ui: &mut egui::Ui,
    track_select: TrackSelect,
    shared: &mut SharedPiyoState,
    n_events: u32,
) {
    let cur = ui.cursor();
    egui::ScrollArea::horizontal().show(ui, |ui| {
        let node_size = 10.0;
        let node_gapped = 16.0;
        let scrollbar_gap = node_gapped * 2.0;
        let content_size = egui::vec2(
            n_events as f32 * node_gapped,
            ((N_KEYS - 1) as f32 * node_gapped) + scrollbar_gap,
        );
        let (rect, re) = ui.allocate_exact_size(content_size, egui::Sense::click_and_drag());
        let clip = ui.clip_rect().intersect(cur);
        let mut x = rect.min.x + node_size;
        let y_off = rect.min.y;
        let p = ui.painter_at(clip);
        let event_cursor = shared.player.event_cursor;
        let paused = shared.paused;
        let events = match track_select {
            TrackSelect::Melody(n) => {
                &mut shared.player.song.melody_tracks[usize::from(n)].base.events
            }
            TrackSelect::Percussion => &mut shared.player.song.percussion_track.base.events,
        };
        let guide_color = ui.style().visuals.widgets.noninteractive.bg_stroke.color;
        let node_color = ui.style().visuals.widgets.hovered.weak_bg_fill;
        for event in &mut *events {
            let mut y = (y_off + ((N_KEYS - 1) as f32 * node_gapped)) + node_gapped / 2.0;
            for key in piano_keys() {
                p.circle_filled(egui::pos2(x, y), 1.0, guide_color);
                if event.key_down(key) {
                    p.rect(
                        egui::Rect::from_center_size(
                            egui::pos2(x, y),
                            egui::vec2(node_size, node_size),
                        ),
                        1.0,
                        node_color,
                        egui::Stroke::new(1.0, guide_color),
                        egui::StrokeKind::Outside,
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
                let pos =
                    pos - rect.min.to_vec2() - egui::vec2(node_gapped / 4.0, node_gapped / 4.0);
                let event = pos / node_gapped;
                let event_off = event.x as usize;
                let key = event.y as PianoKey;
                let off = key + 1;
                let key_idx = N_KEYS.saturating_sub(off);

                match action {
                    Action::Add => {
                        events[event_off].set_key_down(key_idx);
                        play_key(track_select, shared, key_idx);
                    }
                    Action::Del => events[event_off].set_key_up(key_idx),
                    Action::SetPos => shared.player.event_cursor = event_off as u32,
                }
            }
        }
    });
}

// Returns clicked piano key, if any
#[must_use]
fn piano_keys_ui(
    ui: &mut egui::Ui,
    timers: &[f64; N_KEYS as usize],
    max_time: f64,
    key_size: egui::Vec2,
) -> Option<PianoKey> {
    let key_count = timers.len();
    let h = key_size.y * N_KEYS as f32;
    let (outer_rect, re) = ui.allocate_exact_size(egui::vec2(key_size.x, h), egui::Sense::click());
    let painter = ui.painter_at(outer_rect);
    struct Pal {
        bg: egui::Color32,
        white: egui::Color32,
        white_stroke: egui::Color32,
        black: egui::Color32,
        black_stroke: egui::Color32,
        hi: egui::Color32,
    }
    let pal = Pal {
        bg: egui::Color32::from_rgb(124, 69, 183),
        white: egui::Color32::WHITE,
        white_stroke: egui::Color32::DARK_GRAY,
        black: egui::Color32::from_rgb(152, 145, 221),
        black_stroke: egui::Color32::from_rgb(31, 65, 134),
        hi: egui::Color32::from_rgb(26, 218, 108),
    };
    // Split keyboard and grid
    let kb_rect =
        egui::Rect::from_min_size(outer_rect.min, egui::vec2(key_size.x, outer_rect.height()));

    painter.rect_filled(kb_rect, 0.0, pal.bg);

    let is_black = |idx: usize| matches!(idx % 12, 1 | 3 | 6 | 8 | 10);
    let black_w = key_size.x * 0.62;
    let black_h = key_size.y * 0.85;
    let click = ui.input(|inp| inp.pointer.primary_pressed());
    let ipos = re.interact_pointer_pos().filter(|_| click);
    let mut clicked = None;

    for (i, &t) in timers.iter().enumerate() {
        if is_black(i) {
            let k_top = (key_count - 1 - i) as f32;
            let y0 = kb_rect.top() + k_top * key_size.y;
            let x0 = kb_rect.left() - 2.0;
            // FIXME: Proper centering for black keys
            let offset = 2.0;
            let r = egui::Rect::from_min_max(
                egui::pos2(x0, y0 + offset),
                egui::pos2(x0 + black_w, y0 + black_h),
            );
            if let Some(pos) = ipos
                && r.contains(pos)
            {
                clicked = Some(i as PianoKey);
            }
            let ratio = t / max_time;
            let fill = pal.black.lerp_to_gamma(pal.hi, ratio as f32);
            painter.rect(
                r,
                3.0,
                fill,
                egui::Stroke::new(1.0, pal.black_stroke),
                egui::StrokeKind::Outside,
            );
        } else {
            let k_top = (key_count - 1 - i) as f32;
            let y0 = kb_rect.top() + k_top * key_size.y;
            let r = egui::Rect::from_min_max(
                egui::pos2(kb_rect.left() - 2.0, y0),
                egui::pos2(kb_rect.right(), y0 + key_size.y),
            );
            if let Some(pos) = ipos
                && r.contains(pos)
            {
                clicked = Some(i as PianoKey);
            }
            let ratio = t / max_time;
            let fill = pal.white.lerp_to_gamma(pal.hi, ratio as f32);
            painter.rect(
                r,
                2.0,
                fill,
                egui::Stroke::new(1.0, pal.white_stroke),
                egui::StrokeKind::Outside,
            );
        }
    }
    clicked
}
