use crate::editor::theme;
use crate::engine::ComponentType;

use super::Editor;

impl Editor {
    // Helper to draw the unified properties window
    pub(crate) fn draw_properties_ui(&mut self, ui: &mut egui::Ui) {
        if !self.canvas.inspection_path.is_empty() {
            if let Some((blueprint, _)) = self.get_inspected_blueprint_and_components() {
                ui.heading(format!("{} Inspecting Block", theme::ICON_SETTINGS));
                ui.colored_label(egui::Color32::from_rgb(0, 180, 255), &blueprint.name);
                ui.separator();

                egui::Grid::new("inspection_grid")
                    .num_columns(2)
                    .spacing([10.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Inputs:");
                        ui.label(format!("{}", blueprint.inputs));
                        ui.end_row();
                        ui.label("Outputs:");
                        ui.label(format!("{}", blueprint.outputs));
                        ui.end_row();
                        ui.label("Internal Gates:");
                        ui.label(format!("{}", blueprint.components.len()));
                        ui.end_row();
                    });

                ui.add_space(10.0);
                if ui.button("← Exit Inspection").clicked() {
                    self.canvas.inspection_path.pop();
                    if let Some((stashed_pan, stashed_zoom)) =
                        self.canvas.inspection_camera_stack.pop()
                    {
                        self.canvas.pan = stashed_pan;
                        self.canvas.zoom = stashed_zoom;
                    }
                    self.canvas.selected_comp_id = None;
                }
            }
        } else {
            // Delete Selected Button (Touch / UI alternative to Delete key)
            if !self.canvas.selected_comp_ids.is_empty()
                || !self.canvas.selected_connections.is_empty()
            {
                if ui
                    .button(format!(
                        "{} Delete Selected",
                        crate::editor::theme::ICON_DELETE
                    ))
                    .clicked()
                {
                    self.push_history_snapshot();
                    self.components
                        .retain(|c| !self.canvas.selected_comp_ids.contains(&c.id));
                    self.connections.retain(|c| {
                        !self.canvas.selected_comp_ids.contains(&c.src_comp_id)
                            && !self.canvas.selected_comp_ids.contains(&c.tgt_comp_id)
                            && !self.canvas.selected_connections.contains(c)
                    });
                    self.canvas.selected_comp_ids.clear();
                    self.canvas.selected_connections.clear();
                    self.canvas.selected_comp_id = None;
                    self.compile();
                }
                ui.separator();
            }

            // If a component is selected, allow inspecting & editing properties
            let mut has_selection = false;

            let mut trigger_history = false;
            let mut new_label: Option<String> = None;
            let mut new_period: Option<usize> = None;
            let mut new_bus_width: Option<usize> = None;
            let mut do_inspection = false;

            if let Some(sel_id) = self.canvas.selected_comp_id {
                let mut comp_clone = None;
                for c in &self.components {
                    if c.id == sel_id {
                        comp_clone = Some(c.clone());
                        break;
                    }
                }

                if let Some(comp) = comp_clone {
                    has_selection = true;
                    ui.heading(format!("{} Component", theme::ICON_SETTINGS));
                    ui.add_space(5.0);

                    egui::CollapsingHeader::new("General")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::Grid::new("comp_general_grid")
                                .num_columns(2)
                                .spacing([10.0, 10.0])
                                .show(ui, |ui| {
                                    ui.label("Type:");
                                    ui.label(format!("{:?}", comp.comp_type));
                                    ui.end_row();

                                    ui.label("Label:");
                                    let mut label = comp.label.clone();
                                    if ui.text_edit_singleline(&mut label).changed() {
                                        new_label = Some(label);
                                    }
                                    ui.end_row();
                                });
                        });

                    if let ComponentType::Clock = comp.comp_type {
                        ui.add_space(5.0);
                        egui::CollapsingHeader::new("Simulation")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new("comp_sim_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 10.0])
                                    .show(ui, |ui| {
                                        ui.label("Period:");
                                        let mut period = comp.clock_period.unwrap_or(20);
                                        let response = ui.add(
                                            egui::Slider::new(&mut period, 2..=1000).text("ticks"),
                                        );
                                        if response.drag_started() {
                                            trigger_history = true;
                                        }
                                        if response.changed() {
                                            new_period = Some(period);
                                        }
                                        ui.end_row();
                                    });
                            });
                    }

                    if let ComponentType::BusJoiner | ComponentType::BusSplitter = comp.comp_type {
                        ui.add_space(5.0);
                        egui::CollapsingHeader::new("Bus Properties")
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new("comp_bus_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 10.0])
                                    .show(ui, |ui| {
                                        ui.label("Width (bits):");
                                        let mut width = comp.bus_width();
                                        let response = ui.add(
                                            egui::Slider::new(&mut width, 2..=16).text("bits"),
                                        );
                                        if response.drag_started() {
                                            trigger_history = true;
                                        }
                                        if response.changed() {
                                            new_bus_width = Some(width);
                                        }
                                        ui.end_row();
                                    });
                            });
                    }

                    if let ComponentType::SubChip(bp_idx) = comp.comp_type {
                        ui.add_space(5.0);
                        egui::CollapsingHeader::new("Actions")
                            .default_open(true)
                            .show(ui, |ui| {
                                if ui
                                    .button(format!("{} Look Inside", theme::ICON_FOLDER))
                                    .clicked()
                                {
                                    do_inspection = true;
                                }
                                if ui
                                    .button(format!("{} Edit Blueprint", theme::ICON_EDIT))
                                    .clicked()
                                {
                                    self.unpack_blueprint_to_canvas(bp_idx);
                                }
                            });
                    }

                    ui.add_space(5.0);
                    ui.separator();
                }
            }

            if trigger_history {
                self.push_history_snapshot();
            }

            if do_inspection && let Some(sel_id) = self.canvas.selected_comp_id {
                self.canvas
                    .inspection_camera_stack
                    .push((self.canvas.pan, self.canvas.zoom));
                self.canvas.inspection_path.push(sel_id);
                self.canvas.selected_comp_id = None;
                self.center_camera_on_inspection_view();
            }

            if (new_label.is_some() || new_period.is_some() || new_bus_width.is_some())
                && let Some(sel_id) = self.canvas.selected_comp_id
            {
                let mut size_changed = false;
                for c in &mut self.components {
                    if c.id == sel_id {
                        if let Some(ref l) = new_label {
                            c.label = l.clone();
                        }
                        if let Some(p) = new_period {
                            c.clock_period = Some(p);
                            if let Some(active_clk) = self
                                .engine
                                .active_clocks
                                .iter_mut()
                                .find(|ac| ac.visual_id == Some(sel_id))
                            {
                                active_clk.period = p;
                            }
                        }
                        if let Some(w) = new_bus_width {
                            c.clock_period = Some(w);
                            c.width = 50.0;
                            c.height = 40.0 + (w as f32 * 16.0);
                            size_changed = true;
                        }
                    }
                }

                if size_changed {
                    let (ins, outs) = if let Some(comp) = self.components.iter().find(|c| c.id == sel_id) {
                        self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()))
                    } else {
                        (0, 0)
                    };

                    self.connections.retain(|conn| {
                        let src_ok = if conn.src_comp_id == sel_id {
                            conn.src_port < outs
                        } else {
                            true
                        };
                        let tgt_ok = if conn.tgt_comp_id == sel_id {
                            conn.tgt_port < ins
                        } else {
                            true
                        };
                        src_ok && tgt_ok
                    });
                }

                self.compile();
            }

            let mut delete_annotation_idx = None;
            if let Some(idx) = self.canvas.selected_annotation_idx
                && let Some(ann) = self.annotations.get_mut(idx)
            {
                has_selection = true;
                egui::CollapsingHeader::new(format!("{} Text Note", theme::ICON_SETTINGS))
                    .default_open(true)
                    .show(ui, |ui| {
                        let response = ui.text_edit_multiline(&mut ann.text);
                        if self.canvas.focus_annotation_text {
                            response.request_focus();
                            self.canvas.focus_annotation_text = false;
                        }
                    });
                ui.add_space(5.0);
                if ui
                    .button(format!("{} Delete Note", crate::editor::theme::ICON_DELETE))
                    .clicked()
                {
                    self.push_history_snapshot();
                    delete_annotation_idx = Some(idx);
                }
                ui.separator();
            }

            if let Some(idx) = delete_annotation_idx {
                self.annotations.remove(idx);
                self.canvas.selected_annotation_idx = None;
            }

            if !has_selection {
                egui::CollapsingHeader::new("Canvas Settings")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("canvas_settings_grid")
                            .num_columns(2)
                            .spacing([10.0, 10.0])
                            .show(ui, |ui| {
                                ui.label("Chip Name:");
                                ui.text_edit_singleline(&mut self.ui.chip_name_input);
                                ui.end_row();
                            });

                        ui.add_space(5.0);
                        if ui
                            .button(format!("{} Compile Chip", theme::ICON_SAVE))
                            .clicked()
                            && let Some(new_bp) = self.package_current_canvas()
                        {
                            self.push_history_snapshot();
                            // Add to global library and sync
                            self.global_library.ungrouped.push(new_bp);
                            crate::editor::global_library::save_global_library(&self.global_library);
                            self.engine.library = self.global_library.to_flat_list();
                            self.components.clear();
                            self.connections.clear();
                            self.compile();
                        }
                    });

                ui.add_space(5.0);
                egui::CollapsingHeader::new("Actions")
                    .default_open(true)
                    .show(ui, |ui| {
                        if ui
                            .button(format!(
                                "{} Clear Canvas",
                                crate::editor::theme::ICON_DELETE
                            ))
                            .clicked()
                        {
                            self.push_history_snapshot();
                            self.components.clear();
                            self.connections.clear();
                            self.compile();
                        }
                        if ui
                            .button(format!(
                                "{} Recompile Graph",
                                crate::editor::theme::ICON_REFRESH
                            ))
                            .clicked()
                        {
                            self.compile();
                        }
                    });
            }
        }
    }
}
