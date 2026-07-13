#![windows_subsystem = "windows"]
#![allow(clippy::type_complexity)]

pub mod editor;
pub mod engine;

use editor::Editor;
use macroquad::prelude::*;

pub async fn run() {
    let mut editor = Editor::new();
    editor::gui::setup_egui();
    egui_macroquad::draw(); // Flush the setup frame before the main loop starts

    loop {
        // Process egui UI logic first (updates egui_wants_pointer for the current frame)
        editor.draw_gui();

        // Update logic editor state using the fresh input state
        editor.update();

        // Draw 2D logic canvas (wires, gates)
        editor.draw();

        // Render egui panels on top of the canvas
        egui_macroquad::draw();

        // Wait for next frame
        next_frame().await;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Digital Logic Simulator".to_string(),
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
pub async fn main() {
    run().await;
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn quad_main() {
    main();
}
