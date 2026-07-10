use macroquad::prelude::*;
use super::Editor;

impl Editor {
    pub fn handle_canvas_interactions(
        &mut self,
        mouse_pos_world: Vec2,
        mouse_delta: Vec2,
        egui_wants_pointer: bool,
    ) {
        let touch_events = touches();
        let is_multi_touch = touch_events.len() >= 2;

        if !egui_wants_pointer && !is_multi_touch && self.canvas.inspection_path.is_empty() {
            if is_mouse_button_pressed(MouseButton::Left) {
                self.handle_canvas_left_press(mouse_pos_world);
            } else if is_mouse_button_down(MouseButton::Left) {
                self.handle_canvas_left_drag(mouse_pos_world, mouse_delta);
            } else if is_mouse_button_released(MouseButton::Left) {
                self.handle_canvas_left_release(mouse_pos_world);
            }
        }

        if !egui_wants_pointer
            && self.canvas.inspection_path.is_empty()
            && (is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace))
        {
            self.handle_canvas_deletion();
        }
    }
}
