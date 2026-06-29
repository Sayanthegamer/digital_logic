use crate::engine::ComponentType;

use super::Editor;

impl Editor {
    pub fn draw_gui(&mut self) {
        let mut egui_wants_pointer = false;
        egui_macroquad::ui(|ctx| {
            ctx.set_pixels_per_point(self.ui_scale);
            egui_wants_pointer = ctx.wants_pointer_input() || ctx.wants_keyboard_input();
            // Dark elegant theme styling overrides
            let mut style = (*ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(32, 60, 48);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(36, 42, 45);
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(18, 20, 22);
            ctx.set_style(style);

            // Error panel if simulation has oscillated/errored
            if let Some(ref err) = self.propagation_error {
                egui::TopBottomPanel::top("error_panel").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "⚠️ Simulation Error:");
                        ui.label(err);
                    });
                });
            }

            // 1. Sidebar catalog panel
            if self.inspection_path.is_empty() {
                egui::SidePanel::left("parts_catalog")
                    .resizable(false)
                    .default_width(180.0)
                    .show(ctx, |ui| {
                        ui.add_space(10.0);
                        ui.heading("Parts Catalog");
                        ui.separator();
                        ui.add_space(5.0);

                        // Library Primitives
                        ui.label("Primitives");
                        if ui.selectable_label(self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input)), "Input Pin").clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input));
                        }
                        if ui.selectable_label(self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output)), "Output Pin").clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output));
                        }
                        if ui.selectable_label(self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand)), "NAND Gate").clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand));
                        }
                        if ui.selectable_label(self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Clock)), "Clock Input").clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Clock));
                        }
                        if ui.selectable_label(self.selected_tool == Some(super::types::ActiveTool::PlaceAnnotation), "Text Annotation").clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceAnnotation);
                        }
                        
                        if ui.button("Clear Selection").clicked() {
                            self.selected_tool = None;
                        }

                        ui.add_space(20.0);
                        ui.label("Custom Chips");
                        ui.separator();

                        for (idx, bp) in self.library.iter().enumerate() {
                            let is_sel = self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            if ui.selectable_label(is_sel, &bp.name).clicked() {
                                self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            }
                        }
                    });
            }

            // 2. Control Toolbar (Top Panel)
            egui::TopBottomPanel::top("control_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if !self.inspection_path.is_empty() {
                        if ui.button("← Exit Inspection").clicked() {
                            self.inspection_path.pop();
                            self.selected_comp_id = None;
                        }
                        ui.separator();
                    }
                    ui.heading("Digital Logic Sim");
                    ui.add_space(30.0);

                    // Simulation Ticker controls
                    if ui.button(if self.is_playing { "Pause" } else { "Play" }).clicked() {
                        self.is_playing = !self.is_playing;
                    }

                    if ui.button("Step Tick").clicked() {
                        let _ = self.simulator.propagate_events(50);
                    }

                    ui.add_space(20.0);
                    ui.label("Sim Speed:");
                    ui.add(egui::Slider::new(&mut self.ticks_per_frame, 1..=500).text("ticks/frame"));

                    ui.add_space(20.0);
                    if ui.button("Recompile Graph").clicked() {
                        self.compile();
                    }

                    if ui.button("Clear Canvas").clicked() {
                        self.components.clear();
                        self.connections.clear();
                        self.compile();
                    }
                    ui.separator();
                    if ui.button("💾 Save").clicked() {
                        self.save_project();
                    }
                    if ui.button("📂 Load").clicked() {
                        self.load_project();
                    }
                    ui.separator();
                    if ui.button("⚙ Settings").clicked() {
                        self.show_settings = !self.show_settings;
                    }
                });
            });

            // 3. Packaging Chip Panel (Right Panel)
            egui::SidePanel::right("package_panel")
                .resizable(false)
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.add_space(10.0);

                    if !self.inspection_path.is_empty() {
                        if let Some((blueprint, _)) = self.get_inspected_blueprint_and_components() {
                            ui.heading("Inspecting Block");
                            ui.colored_label(egui::Color32::from_rgb(0, 180, 255), &blueprint.name);
                            ui.separator();
                            ui.add_space(10.0);
                            
                            ui.label(format!("Inputs: {}", blueprint.inputs));
                            for (i, name) in blueprint.input_names.iter().enumerate() {
                                ui.small(format!("  Pin {}: {}", i, name));
                            }
                            
                            ui.add_space(10.0);
                            ui.label(format!("Outputs: {}", blueprint.outputs));
                            for (o, name) in blueprint.output_names.iter().enumerate() {
                                ui.small(format!("  Pin {}: {}", o, name));
                            }
                            
                            ui.add_space(10.0);
                            ui.label(format!("Internal Gates: {}", blueprint.components.len()));
                            
                            ui.add_space(30.0);
                            if ui.button("← Exit Inspection").clicked() {
                                self.inspection_path.pop();
                                self.selected_comp_id = None;
                            }
                        }
                     } else {
                         // Delete Selected Button (Touch / UI alternative to Delete key)
                         if !self.selected_comp_ids.is_empty() {
                             ui.add_space(5.0);
                             if ui.button("🗑 Delete Selected").clicked() {
                                 self.components.retain(|c| !self.selected_comp_ids.contains(&c.id));
                                 self.connections.retain(|c| !self.selected_comp_ids.contains(&c.src_comp_id) && !self.selected_comp_ids.contains(&c.tgt_comp_id));
                                 self.selected_comp_ids.clear();
                                 self.selected_comp_id = None;
                                 self.compile();
                             }
                             ui.separator();
                             ui.add_space(5.0);
                         }

                         // If a component is selected, allow inspecting & editing properties
                         if let Some(sel_id) = self.selected_comp_id {
                             let mut comp_opt = None;
                             for c in &mut self.components {
                                 if c.id == sel_id {
                                     comp_opt = Some(c);
                                     break;
                                 }
                             }
                             
                             if let Some(comp) = comp_opt {
                                 ui.heading("Selected Component");
                                 ui.label(format!("ID: {}", comp.id));
                                 ui.label(format!("Type: {:?}", comp.comp_type));
                                 
                                 ui.add_space(5.0);
                                 ui.label("Label / Name:");
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
                                     ui.label("Clock Period (ticks):");
                                     if ui.add(egui::Slider::new(&mut period, 2..=1000).text("ticks")).changed() {
                                         comp.clock_period = Some(period);
                                         // Directly update the compiled active_clocks array!
                                         if let Some(active_clk) = self.active_clocks.iter_mut().find(|ac| ac.visual_id == Some(comp.id)) {
                                             active_clk.period = period;
                                         }
                                     }
                                 }
                                 
                                 ui.separator();
                                 ui.add_space(15.0);
                             }
                         }

                         if let Some(idx) = self.selected_annotation_idx
                             && let Some(ann) = self.annotations.get_mut(idx) {
                                 ui.heading("Selected Text Label");
                                 ui.label("Text Content:");
                                 ui.text_edit_multiline(&mut ann.text);
                                 
                                 ui.separator();
                                 ui.add_space(15.0);
                             }

                        ui.heading("Package Chip");
                        ui.separator();
                        ui.add_space(5.0);
                        
                        ui.label("Create a reusable custom block out of your current canvas layout.");
                        ui.add_space(10.0);

                        ui.label("Chip Name:");
                        ui.text_edit_singleline(&mut self.chip_name_input);

                        ui.add_space(15.0);

                        if ui.button("Compile & Save to Catalog").clicked()
                            && let Some(new_bp) = self.package_current_canvas() {
                                self.library.push(new_bp);
                                self.components.clear();
                                self.connections.clear();
                                self.compile();
                            }
                    }
                });

            if self.show_settings {
                let mut show_settings = self.show_settings;
                egui::Window::new("Settings")
                    .open(&mut show_settings)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.heading("Graphics Settings");
                        ui.add_space(5.0);

                        // 1. Fullscreen Toggle
                        let mut is_fullscreen = self.is_fullscreen;
                        if ui.checkbox(&mut is_fullscreen, "Fullscreen").changed() {
                            self.is_fullscreen = is_fullscreen;
                            macroquad::window::set_fullscreen(is_fullscreen);
                        }

                        ui.add_space(5.0);

                        // 2. Resolution Choice
                        ui.label("Window Resolution:");
                        let resolutions = &[
                            (800, 600, "800 x 600"),
                            (1024, 768, "1024 x 768"),
                            (1280, 720, "1280 x 720 (Default)"),
                            (1600, 900, "1600 x 900"),
                            (1920, 1080, "1920 x 1080"),
                        ];
                        let mut selected_idx = self.resolution_idx;
                        egui::ComboBox::from_label("")
                            .selected_text(resolutions[selected_idx].2)
                            .show_ui(ui, |ui| {
                                for (idx, r) in resolutions.iter().enumerate() {
                                    ui.selectable_value(&mut selected_idx, idx, r.2);
                                }
                            });
                        if selected_idx != self.resolution_idx {
                            self.resolution_idx = selected_idx;
                            let r = resolutions[selected_idx];
                            macroquad::window::request_new_screen_size(r.0 as f32, r.1 as f32);
                        }

                        ui.add_space(10.0);
                        ui.heading("UI Scaling");
                        ui.add_space(5.0);

                        // 3. UI Scale (zoom factor)
                        let mut ui_scale = self.ui_scale;
                        ui.add(egui::Slider::new(&mut ui_scale, 0.5..=2.0).text("UI Scale"));
                        if ui_scale != self.ui_scale {
                            self.ui_scale = ui_scale;
                        }
                    });
                self.show_settings = show_settings;
            }
        });
        self.egui_wants_pointer = egui_wants_pointer;
    }
}
