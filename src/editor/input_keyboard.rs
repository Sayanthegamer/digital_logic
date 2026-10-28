use super::Editor;
use super::types::ActiveTool;
use crate::editor::math::Vec2;
use crate::engine::ComponentType;

impl Editor {
    pub fn handle_keyboard_shortcuts(&mut self, egui_wants_keyboard: bool) {
        if !egui_wants_keyboard {
            let fi = self.canvas.frame_input.clone();

            if fi.space_pressed {
                self.engine.is_playing = !self.engine.is_playing;
            }
            if fi.c_pressed {
                self.compile();
            }
            if fi.escape_pressed {
                self.canvas.selected_tool = None;
                self.canvas.selected_comp_id = None;
            }
            if fi.ctrl_down && fi.s_pressed {
                self.save_project();
            }
            if fi.ctrl_down && fi.l_pressed {
                self.load_project();
            }
            if fi.g_pressed {
                println!("--- STARTING GRID VERIFICATION ---");
                self.verify_grid();
                println!("--- FINISHED GRID VERIFICATION ---");
            }
            if fi.f11_pressed {
                self.generate_hot_reload_test();
            }
            if fi.f12_pressed {
                self.generate_stress_test(self.ui.stress_test_size);
            }
            if fi.f6_pressed {
                self.ui.show_debug_suite = !self.ui.show_debug_suite;
            }

            // Tool Shortcuts
            if fi.key1_or_i_pressed {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Input));
            }
            if fi.key2_or_o_pressed {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Output));
            }
            if fi.key3_or_n_pressed {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Nand));
            }
            if fi.key4_or_k_pressed {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Clock));
            }
            if fi.key5_or_t_pressed {
                self.canvas.selected_tool = Some(ActiveTool::PlaceAnnotation);
            }

            // Keyboard Navigation (Tab / Shift+Tab)
            if fi.tab_pressed {
                self.advance_tab_focus(fi.shift_down);
            }

            // Keyboard Interaction (Enter)
            if fi.enter_pressed
                && let Some((comp_id, port_opt)) = self.canvas.tab_focus
            {
                if let Some((port_idx, is_input)) = port_opt {
                    self.canvas.hovered_port = Some((comp_id, port_idx, is_input));
                    if let Some(comp) = self.get_component(comp_id) {
                        let (in_count, out_count) = self.get_component_ports_count_with_width(
                            comp.comp_type,
                            Some(comp.bus_width()),
                        );
                        let pos = if is_input {
                            comp.input_port_pos(port_idx, in_count)
                        } else {
                            comp.output_port_pos(port_idx, out_count)
                        };
                        self.handle_canvas_left_press(pos);
                    }
                } else if let Some(comp) = self.get_component(comp_id)
                    && comp.comp_type == ComponentType::Input
                    && let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&comp_id)
                {
                    let curr_val = self.engine.simulator.get_state(gate_idx);
                    self.engine.simulator.set_input(gate_idx, !curr_val);
                }
            }
        }
    }

    fn advance_tab_focus(&mut self, reverse: bool) {
        if self.circuit.components.is_empty() {
            self.canvas.tab_focus = None;
            return;
        }

        let mut focusable = Vec::new();
        for comp in &self.circuit.components {
            focusable.push((comp.id, None));
            let (in_count, out_count) =
                self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));
            for i in 0..in_count {
                focusable.push((comp.id, Some((i, true))));
            }
            for i in 0..out_count {
                focusable.push((comp.id, Some((i, false))));
            }
        }

        let current_idx = if let Some(focus) = self.canvas.tab_focus {
            focusable.iter().position(|&x| x == focus).unwrap_or(0)
        } else if reverse {
            focusable.len() - 1
        } else {
            0
        };

        let next_idx = if reverse {
            if current_idx == 0 {
                focusable.len() - 1
            } else {
                current_idx - 1
            }
        } else if current_idx + 1 >= focusable.len() {
            0
        } else {
            current_idx + 1
        };

        self.canvas.tab_focus = Some(focusable[next_idx]);

        if let Some((comp_id, _)) = self.canvas.tab_focus
            && let Some(comp) = self.get_component(comp_id)
        {
            let center = comp.pos + Vec2::new(comp.width / 2.0, comp.height / 2.0);
            self.canvas.pan = Vec2::new(self.screen_width() / 2.0, self.screen_height() / 2.0)
                - center * self.canvas.zoom;
        }
    }
}
