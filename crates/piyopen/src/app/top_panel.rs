use {
    crate::app::{FileDialogOp, PiyopenApp, SharedPiyoState},
    eframe::egui,
    std::path::Path,
};

pub fn ui(app: &mut PiyopenApp, ui: &mut egui::Ui, space_key_down: bool) {
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Open").clicked() {
                if let Some(path) = &app.open_path
                    && let Some(parent) = <_ as AsRef<Path>>::as_ref(path).parent()
                {
                    app.file_dia.config_mut().initial_directory = parent.to_path_buf();
                }
                app.file_dia.pick_file();
                app.file_dia.set_user_data(FileDialogOp::OpenFile);
            }
            if let Some(shared) = &mut app.shared
                && let Some(path) = &app.open_path
                && ui.button("Reload").clicked()
            {
                match SharedPiyoState::new(path) {
                    Ok(new) => *shared.lock() = new,
                    Err(e) => app.popup_msg = Some(e.to_string()),
                }
            }
            if ui.button("🗛 Add fallback font").clicked() {
                app.file_dia.pick_file();
                app.file_dia.set_user_data(FileDialogOp::AddFont);
            }
        });
        if let Some(shared) = app.shared.as_mut() {
            ui.separator();
            let mut shared = shared.lock();
            let label = if shared.paused { "▶" } else { "⏸" };
            if ui.button(label).on_hover_text("Play/Pause").clicked() || space_key_down {
                shared.paused ^= true;
            }
            if ui.button("⏮").on_hover_text("Seek to beginning").clicked() {
                shared.player.event_cursor = 0;
            }
            if ui
                .button("⟲")
                .on_hover_text("Seek to repeat point")
                .clicked()
            {
                shared.player.event_cursor = shared.player.song.repeat_range.start;
            }
            ui.label("🔉");
            ui.add(egui::Slider::new(&mut shared.volume, 0.0..=1.0));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(path) = &app.open_path {
                ui.label(path);
            }
        });
    });
}
