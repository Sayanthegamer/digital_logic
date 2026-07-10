use crate::engine::ComponentType;
use macroquad::prelude::*;
use super::types::ActiveTool;
use super::Editor;

impl Editor {
    pub fn handle_keyboard_shortcuts(&mut self, egui_wants_keyboard: bool) {
        if !egui_wants_keyboard {
            if is_key_pressed(KeyCode::Space) {
                self.engine.is_playing = !self.engine.is_playing;
            }
            if is_key_pressed(KeyCode::C) {
                self.compile();
            }
            if is_key_pressed(KeyCode::Escape) {
                self.canvas.selected_tool = None;
                self.canvas.selected_comp_id = None;
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
                && is_key_pressed(KeyCode::S)
            {
                self.save_project();
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
                && is_key_pressed(KeyCode::L)
            {
                self.load_project();
            }

            // Tool Shortcuts
            if is_key_pressed(KeyCode::Key1) || is_key_pressed(KeyCode::I) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Input));
            }
            if is_key_pressed(KeyCode::Key2) || is_key_pressed(KeyCode::O) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Output));
            }
            if is_key_pressed(KeyCode::Key3) || is_key_pressed(KeyCode::N) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Nand));
            }
            if is_key_pressed(KeyCode::Key4) || is_key_pressed(KeyCode::K) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Clock));
            }
            if is_key_pressed(KeyCode::Key5) || is_key_pressed(KeyCode::T) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceAnnotation);
            }
        }
    }
}
