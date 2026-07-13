use macroquad::prelude::*;
use super::Editor;

impl Editor {
    pub fn to_world_space(&self, screen_pos: Vec2) -> Vec2 {
        (screen_pos - self.canvas.pan) / self.canvas.zoom
    }

    pub fn to_screen_space(&self, world_pos: Vec2) -> Vec2 {
        world_pos * self.canvas.zoom + self.canvas.pan
    }

    pub fn update(&mut self) {
        if self.ui.mode != crate::editor::state::AppMode::Editor {
            // Reset right drag tracking when entering/leaving menus
            self.canvas.right_drag_dist = 0.0;
            return;
        }

        let mouse_pos_screen = mouse_position().into();
        let mouse_pos_world = self.to_world_space(mouse_pos_screen);

        let mouse_delta = if self.canvas.last_mouse_pos == Vec2::ZERO {
            Vec2::ZERO
        } else {
            mouse_pos_screen - self.canvas.last_mouse_pos
        };

        let egui_wants_pointer = self.ui.egui_wants_pointer;
        let egui_wants_keyboard = self.ui.egui_wants_keyboard;

        // 1. Touch Input Abstraction (Mobile)
        self.handle_touch_input(egui_wants_pointer);

        // 2. Keyboard & Tool Shortcuts
        self.handle_keyboard_shortcuts(egui_wants_keyboard);

        // 3. Magnetic Port Hover Detection
        self.update_hovered_port(mouse_pos_screen, mouse_pos_world, egui_wants_pointer);

        // 4. Zoom with mouse wheel
        self.handle_mouse_zoom(mouse_pos_screen, egui_wants_pointer);

        // 5. Pan with right drag
        self.handle_right_drag_pan(mouse_delta, egui_wants_pointer);

        // 6. Interactions: Left click / drag
        self.handle_canvas_interactions(mouse_pos_world, mouse_delta, egui_wants_pointer);

        // 7. Run continuous simulation ticks
        self.run_simulation_ticks();

        // 8. Resolution Change Revert Timer
        self.update_resolution_revert_timer();

        if self.canvas.dragging_comp_id.is_some() || !self.canvas.drag_start_positions.is_empty() {
            if mouse_delta.length_squared() > 0.0 {
                let mut affected = std::collections::HashSet::new();
                if let Some(id) = self.canvas.dragging_comp_id {
                    affected.insert(id);
                }
                for &id in self.canvas.drag_start_positions.keys() {
                    affected.insert(id);
                }
                self.recompute_wire_offsets(Some(&affected));
            }
        }

        self.canvas.last_mouse_pos = mouse_pos_screen;

        // 9. Right-click context menu detection
        self.handle_right_click_context_menu(mouse_pos_world, egui_wants_pointer);
    }
}
