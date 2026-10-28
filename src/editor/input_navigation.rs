use super::Editor;
use crate::editor::math::Vec2;

impl Editor {
    pub fn handle_touch_input(&mut self, egui_wants_pointer: bool) {
        let touch_points = &self.canvas.frame_input.touch_points;
        let is_multi_touch = touch_points.len() >= 2;

        if !egui_wants_pointer && is_multi_touch {
            let t1 = touch_points[0];
            let t2 = touch_points[1];

            let current_dist = t1.distance(t2);
            let current_center = (t1 + t2) * 0.5;

            if let Some(last_dist) = self.canvas.last_touch_dist
                && last_dist > 0.0
            {
                let prev_zoom = self.canvas.zoom;
                self.canvas.zoom *= current_dist / last_dist;
                self.canvas.zoom = self.canvas.zoom.clamp(0.15, 4.0);
                self.canvas.pan = current_center
                    - (current_center - self.canvas.pan) * (self.canvas.zoom / prev_zoom);
            }

            if let Some(last_center) = self.canvas.last_touch_center {
                let delta = current_center - last_center;
                self.canvas.pan += delta;
            }

            self.canvas.last_touch_dist = Some(current_dist);
            self.canvas.last_touch_center = Some(current_center);

            self.canvas.dragging_comp_id = None;
            self.canvas.active_wire_drag = None;
            self.canvas.selection_box_start = None;
        } else {
            self.canvas.last_touch_dist = None;
            self.canvas.last_touch_center = None;
        }
    }

    pub fn handle_mouse_zoom(&mut self, mouse_pos_screen: Vec2, egui_wants_pointer: bool) {
        if !egui_wants_pointer {
            let scroll = self.canvas.frame_input.scroll_delta_y;
            if scroll != 0.0 {
                let prev_zoom = self.canvas.zoom;
                if scroll > 0.0 {
                    self.canvas.zoom *= 1.15;
                } else {
                    self.canvas.zoom /= 1.15;
                }
                self.canvas.zoom = self.canvas.zoom.clamp(0.01, 4.0);

                self.canvas.pan = mouse_pos_screen
                    - (mouse_pos_screen - self.canvas.pan) * (self.canvas.zoom / prev_zoom);
            }
        }
    }

    pub fn handle_right_drag_pan(&mut self, mouse_delta: Vec2, egui_wants_pointer: bool) {
        if !egui_wants_pointer && self.canvas.frame_input.right_down {
            self.canvas.pan += mouse_delta;
            self.canvas.selected_tool = None;
            self.canvas.right_drag_dist += mouse_delta.length();
        }
        if !egui_wants_pointer && self.canvas.frame_input.right_pressed {
            self.canvas.right_drag_dist = 0.0;
        }
    }
}
