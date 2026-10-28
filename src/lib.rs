#![windows_subsystem = "windows"]
#![allow(clippy::type_complexity)]

pub mod editor;
pub mod engine;

use editor::Editor;

impl eframe::App for Editor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.step_frame(ctx);
        ctx.request_repaint();
    }
}

pub fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Digital Logic Simulator")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Digital Logic Simulator",
        native_options,
        Box::new(|cc| {
            editor::gui::setup_egui_context(&cc.egui_ctx);
            Ok(Box::new(Editor::new()))
        }),
    )
}
