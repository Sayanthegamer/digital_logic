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
            if is_key_pressed(KeyCode::F11) {
                self.generate_hot_reload_test();
            }
            if is_key_pressed(KeyCode::F12) {
                self.generate_stress_test();
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

            // Keyboard Navigation (Tab / Shift+Tab)
            if is_key_pressed(KeyCode::Tab) {
                let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                self.advance_tab_focus(shift);
            }

            // Keyboard Interaction (Enter)
            if is_key_pressed(KeyCode::Enter) {
                if let Some((comp_id, port_opt)) = self.canvas.tab_focus {
                    if let Some((port_idx, is_input)) = port_opt {
                        // Enter on a port: start or finish wiring
                        self.canvas.hovered_port = Some((comp_id, port_idx, is_input));
                        if let Some(comp) = self.get_component(comp_id) {
                            let (in_count, out_count) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));
                            let pos = if is_input {
                                comp.input_port_pos(port_idx, in_count)
                            } else {
                                comp.output_port_pos(port_idx, out_count)
                            };
                            self.handle_canvas_left_press(pos);
                        }
                    } else {
                        // Enter on a component: Toggle if it's an Input
                        if let Some(comp) = self.get_component(comp_id) {
                            if comp.comp_type == ComponentType::Input {
                                if let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&comp_id) {
                                    let curr_val = self.engine.simulator.get_state(gate_idx);
                                    self.engine.simulator.set_input(gate_idx, !curr_val);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn advance_tab_focus(&mut self, reverse: bool) {
        if self.circuit.components.is_empty() {
            self.canvas.tab_focus = None;
            return;
        }

        // Build a flat list of focusable elements: (comp_id, None) -> (comp_id, Some((0, true))) ...
        let mut focusable = Vec::new();
        for comp in &self.circuit.components {
            focusable.push((comp.id, None));
            let (in_count, out_count) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));
            for i in 0..in_count {
                focusable.push((comp.id, Some((i, true))));
            }
            for i in 0..out_count {
                focusable.push((comp.id, Some((i, false))));
            }
        }

        let current_idx = if let Some(focus) = self.canvas.tab_focus {
            focusable.iter().position(|&x| x == focus).unwrap_or(0)
        } else {
            if reverse { focusable.len() - 1 } else { 0 }
        };

        let next_idx = if reverse {
            if current_idx == 0 { focusable.len() - 1 } else { current_idx - 1 }
        } else {
            if current_idx + 1 >= focusable.len() { 0 } else { current_idx + 1 }
        };

        self.canvas.tab_focus = Some(focusable[next_idx]);
        
        // Auto-pan to focused element
        if let Some((comp_id, _)) = self.canvas.tab_focus {
            if let Some(comp) = self.get_component(comp_id) {
                let center = comp.pos + Vec2::new(comp.width / 2.0, comp.height / 2.0);
                self.canvas.pan = Vec2::new(screen_width() / 2.0, screen_height() / 2.0) - center * self.canvas.zoom;
            }
        }
    }
}
