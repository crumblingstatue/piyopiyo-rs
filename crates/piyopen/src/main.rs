use {
    crate::{app::PiyopenApp, config::Config},
    eframe::{
        egui,
        epaint::text::{FontInsert, FontPriority, InsertFontFamily},
    },
    std::path::Path,
};

mod app;
mod config;
mod draw_widgets;

fn add_fallback_font_to_egui(ctx: &egui::Context, name: &str, path: &Path) -> anyhow::Result<()> {
    let data = std::fs::read(path)?;
    let data = egui::FontData::from_owned(data);
    ctx.add_font(FontInsert::new(
        name,
        data,
        vec![InsertFontFamily {
            family: egui::FontFamily::Proportional,
            priority: FontPriority::Lowest,
        }],
    ));
    Ok(())
}

fn main() {
    let mut native_opts = eframe::NativeOptions::default();
    native_opts.viewport.inner_size = Some(egui::vec2(960., 720.));
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
            let cfg = match crate::config::load() {
                Some(cfg) => cfg?,
                None => {
                    eprintln!("Config file doesn't exist. Creating.");
                    Config::default()
                }
            };
            for fallback in &cfg.fallback_fonts {
                if let Err(e) =
                    add_fallback_font_to_egui(&cc.egui_ctx, &fallback.name, fallback.path.as_ref())
                {
                    eprintln!("failed to add fallback font: {e}");
                }
            }
            let path = match &cfg.last_opened {
                Some(path) => Some(path.clone()),
                None => std::env::args().nth(1),
            };
            Ok(Box::new(PiyopenApp::new(path, cfg)?))
        }),
    )
    .unwrap();
}
