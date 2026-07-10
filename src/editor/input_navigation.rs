use macroquad::prelude::*;
use super::Editor;

impl Editor {
    pub fn handle_touch_input(&mut self, egui_wants_pointer: bool) {
        let touch_events = touches();
        let is_multi_touch = touch_events.len() >= 2;

        if !egui_wants_pointer && is_multi_touch {
            let t1 = &touch_events[0];
            let t2 = &touch_events[1];

            let current_dist = t1.position.distance(t2.position);
            let current_center = (t1.position + t2.position) * 0.5;

            if let Some(last_dist) = self.canvas.last_touch_dist
                && last_dist > 0.0
            {
                let prev_zoom = self.canvas.zoom;
                self.canvas.zoom *= current_dist / last_dist;
                self.canvas.zoom = self.canvas.zoom.clamp(0.15, 4.0);
                // Zoom towards the center of the pinch
                self.canvas.pan = current_center
                    - (current_center - self.canvas.pan) * (self.canvas.zoom / prev_zoom);
            }

            if let Some(last_center) = self.canvas.last_touch_center {
                let delta = current_center - last_center;
                self.canvas.pan += delta;
            }

            self.canvas.last_touch_dist = Some(current_dist);
            self.canvas.last_touch_center = Some(current_center);

            // Clear single-touch interactions when multi-touch begins
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
            let scroll = mouse_wheel().1;
            if scroll != 0.0 {
                let prev_zoom = self.canvas.zoom;
                if scroll > 0.0 {
                    self.canvas.zoom *= 1.15;
                } else {
                    self.canvas.zoom /= 1.15;
                }
                self.canvas.zoom = self.canvas.zoom.clamp(0.01, 4.0);

                // Pan adjustment to zoom on mouse cursor
                self.canvas.pan = mouse_pos_screen
                    - (mouse_pos_screen - self.canvas.pan) * (self.canvas.zoom / prev_zoom);
            }
        }
    }

    pub fn handle_right_drag_pan(&mut self, mouse_delta: Vec2, egui_wants_pointer: bool) {
        if !egui_wants_pointer && is_mouse_button_down(MouseButton::Right) {
            self.canvas.pan += mouse_delta;
            self.canvas.selected_tool = None;
            // Track right-drag distance for context menu detection
            self.canvas.right_drag_dist += mouse_delta.length();
        }
        if !egui_wants_pointer && is_mouse_button_pressed(MouseButton::Right) {
            self.canvas.right_drag_dist = 0.0;
        }
    }
}
