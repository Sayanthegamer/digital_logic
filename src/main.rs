pub mod engine;
pub mod editor;

use macroquad::prelude::*;
use editor::Editor;

#[macroquad::main("Digital Logic Simulator")]
async fn main() {
    let mut editor = Editor::new();

    loop {
        // Update logic editor state
        editor.update();

        // Draw 2D logic canvas (wires, gates)
        editor.draw();

        // Render egui panels overlay
        editor.draw_gui();
        egui_macroquad::draw();

        // Wait for next frame
        next_frame().await
    }
}