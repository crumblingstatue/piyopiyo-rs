use crate::app::PiyopenApp;

mod app;

fn main() {
    eframe::run_native(
        "piyopen",
        eframe::NativeOptions::default(),
        Box::new(move |_cc| Ok(Box::new(PiyopenApp::new(std::env::args_os().nth(1))))),
    )
    .unwrap();
}
