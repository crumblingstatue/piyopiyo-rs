use {crate::app::PiyopenApp, eframe::egui};

mod app;
mod draw_widgets;

fn main() {
    let mut native_opts = eframe::NativeOptions::default();
    native_opts.viewport.inner_size = Some(egui::vec2(960., 740.));
    eframe::run_native(
        "piyopen",
        native_opts,
        Box::new(move |cc| {
            cc.egui_ctx.style_mut(|style| {
                let blue = egui::Color32::from_rgb(0, 102, 153);
                let dark_blue = egui::Color32::from_rgb(0, 51, 102);
                let light_blue = egui::Color32::from_rgb(153, 204, 255);
                style.visuals.panel_fill = blue;
                style.visuals.widgets.noninteractive.bg_stroke.color = dark_blue;
                style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::WHITE;
                style.visuals.widgets.inactive.bg_fill = dark_blue;
                style.visuals.widgets.inactive.weak_bg_fill = dark_blue;
                style.visuals.widgets.hovered.bg_fill = light_blue;
                style.visuals.widgets.hovered.weak_bg_fill = light_blue;
                style.visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::WHITE;
            });
            Ok(Box::new(PiyopenApp::new(std::env::args_os().nth(1))))
        }),
    )
    .unwrap();
}
