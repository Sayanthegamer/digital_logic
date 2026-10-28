use super::Editor;
use crate::editor::math::Vec2;

impl Editor {
    pub fn handle_canvas_interactions(
        &mut self,
        mouse_pos_world: Vec2,
        mouse_delta: Vec2,
        egui_wants_pointer: bool,
    ) {
        let is_multi_touch = self.canvas.frame_input.touch_points.len() >= 2;

        if !egui_wants_pointer && !is_multi_touch && self.canvas.inspection_path.is_empty() {
            if self.canvas.frame_input.left_pressed {
                self.handle_canvas_left_press(mouse_pos_world);
            } else if self.canvas.frame_input.left_down {
                self.handle_canvas_left_drag(mouse_pos_world, mouse_delta);
            } else if self.canvas.frame_input.left_released {
                self.handle_canvas_left_release(mouse_pos_world);
            }
        }

        if !egui_wants_pointer
            && self.canvas.inspection_path.is_empty()
            && self.canvas.frame_input.delete_pressed
        {
            self.handle_canvas_deletion();
        }
    }
}
