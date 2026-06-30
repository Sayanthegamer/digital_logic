#![windows_subsystem = "windows"]
#![allow(clippy::type_complexity)]
#![allow(dead_code)]

pub mod editor;
pub mod engine;

use editor::Editor;
use macroquad::prelude::*;

pub async fn run() {
    let mut editor = Editor::new();

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

#[macroquad::main("Digital Logic Simulator")]
async fn main() {
    run().await;
}
