use crate::engine::ComponentType;

use super::Editor;

impl Editor {
    // Helper to draw the unified properties window
    pub(crate) fn draw_properties_ui(&mut self, ui: &mut egui::Ui) {
        if !self.inspection_path.is_empty() {
            if let Some((blueprint, _)) = self.get_inspected_blueprint_and_components() {
                ui.heading("Inspecting Block");
                ui.colored_label(egui::Color32::from_rgb(0, 180, 255), &blueprint.name);
                ui.separator();
                ui.label(format!("Inputs: {}", blueprint.inputs));
                ui.label(format!("Outputs: {}", blueprint.outputs));
                ui.label(format!("Internal Gates: {}", blueprint.components.len()));
                ui.add_space(10.0);
                if ui.button("← Exit Inspection").clicked() {
                    self.inspection_path.pop();
                    self.selected_comp_id = None;
                }
            }
        } else {
            // Delete Selected Button (Touch / UI alternative to Delete key)
            if !self.selected_comp_ids.is_empty() {
                if ui.button("🗑 Delete Selected").clicked() {
                    self.components.retain(|c| !self.selected_comp_ids.contains(&c.id));
                    self.connections.retain(|c| {
                        !self.selected_comp_ids.contains(&c.src_comp_id)
                            && !self.selected_comp_ids.contains(&c.tgt_comp_id)
                    });
                    self.selected_comp_ids.clear();
                    self.selected_comp_id = None;
                    self.compile();
                }
                ui.separator();
            }

            // If a component is selected, allow inspecting & editing properties
            let mut has_selection = false;
            if let Some(sel_id) = self.selected_comp_id {
                let mut comp_opt = None;
                for c in &mut self.components {
                    if c.id == sel_id {
                        comp_opt = Some(c);
                        break;
                    }
                }

                if let Some(comp) = comp_opt {
                    has_selection = true;
                    ui.heading("Component Properties");
                    ui.label(format!("Type: {:?}", comp.comp_type));

                    ui.label("Label:");
                    let mut label = comp.label.clone();
                    if ui.text_edit_singleline(&mut label).changed() {
                        comp.label = label;
                    }

                    if let ComponentType::SubChip(_) = comp.comp_type {
                        ui.add_space(5.0);
                        if ui.button("🔍 Look Inside").clicked() {
                            self.inspection_path.push(comp.id);
                            self.selected_comp_id = None;
                        }
                    }

                    if let ComponentType::Clock = comp.comp_type {
                        ui.add_space(5.0);
                        let mut period = comp.clock_period.unwrap_or(20);
                        ui.label("Period:");
                        if ui.add(egui::Slider::new(&mut period, 2..=1000).text("ticks")).changed() {
                            comp.clock_period = Some(period);
                            if let Some(active_clk) = self.active_clocks.iter_mut().find(|ac| ac.visual_id == Some(comp.id)) {
                                active_clk.period = period;
                            }
                        }
                    }
                    ui.separator();
                }
            }

            let mut delete_annotation_idx = None;
            if let Some(idx) = self.selected_annotation_idx {
                if let Some(ann) = self.annotations.get_mut(idx) {
                    has_selection = true;
                    ui.heading("Text Label");
                    let response = ui.text_edit_multiline(&mut ann.text);
                    if self.focus_annotation_text {
                        response.request_focus();
                        self.focus_annotation_text = false;
                    }
                    ui.add_space(5.0);
                    if ui.button("🗑 Delete Selected").clicked() {
                        delete_annotation_idx = Some(idx);
                    }
                    ui.separator();
                }
            }

            if let Some(idx) = delete_annotation_idx {
                self.annotations.remove(idx);
                self.selected_annotation_idx = None;
            }

            if !has_selection {
                ui.heading("Package Canvas");
                ui.label("Chip Name:");
                ui.text_edit_singleline(&mut self.chip_name_input);
                ui.add_space(5.0);
                if ui.button("📦 Compile Chip").clicked() {
                    if let Some(new_bp) = self.package_current_canvas() {
                        self.library.push(new_bp);
                        self.components.clear();
                        self.connections.clear();
                        self.compile();
                    }
                }
                
                ui.add_space(10.0);
                ui.separator();
                if ui.button("🗑 Clear Canvas").clicked() {
                    self.components.clear();
                    self.connections.clear();
                    self.compile();
                }
                if ui.button("🔄 Recompile Graph").clicked() {
                    self.compile();
                }
            }
        }
    }
}
