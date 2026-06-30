use super::Editor;

impl Editor {
    pub fn draw_gui(&mut self) {
        let mut egui_wants_pointer = false;
        let is_mobile = macroquad::window::screen_width() < 720.0;

        egui_macroquad::ui(|ctx| {
            ctx.set_pixels_per_point(self.ui_scale);
            egui_wants_pointer = ctx.wants_pointer_input() || ctx.wants_keyboard_input();
            
            // Dark elegant theme styling overrides with large touch-friendly targets
            let mut style = (*ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(32, 60, 48);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(36, 42, 45);
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(18, 20, 22);
            style.visuals.window_corner_radius = egui::CornerRadius::same(12);
            style.spacing.button_padding = egui::vec2(14.0, 10.0);
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            ctx.set_style(style);

            // Error panel if simulation has oscillated/errored
            if let Some(ref err) = self.propagation_error {
                egui::TopBottomPanel::top("error_panel").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 80, 80),
                            "⚠️ Simulation Error:",
                        );
                        ui.label(err);
                    });
                });
            }

            if is_mobile {
                // --- MOBILE LAYOUT ---
                // 1. Tiny Transparent FAB for Menu & Play/Pause at the top
                egui::Window::new("Mobile FAB")
                    .anchor(egui::Align2::LEFT_TOP, egui::vec2(15.0, 15.0))
                    .collapsible(false)
                    .title_bar(false)
                    .resizable(false)
                    .frame(egui::Frame::window(&ctx.style()).fill(egui::Color32::from_rgba_unmultiplied(18, 20, 22, 180)))
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("🛠️ Menu").clicked() {
                                self.show_menu_mobile = !self.show_menu_mobile;
                            }
                            ui.separator();
                            if ui.button(if self.is_playing { "⏸" } else { "▶" }).clicked() {
                                self.is_playing = !self.is_playing;
                            }
                            if ui.button("⏭").clicked() {
                                let _ = self.simulator.propagate_events(50);
                            }
                        });
                    });

                // 2. Full Control Drawer (overlay)
                if self.show_menu_mobile {
                    egui::Window::new("Simulator Drawer")
                        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                        .collapsible(false)
                        .title_bar(true)
                        .resizable(false)
                        .default_width(320.0)
                        .show(ctx, |ui| {
                            ui.vertical(|ui| {
                                if !self.inspection_path.is_empty() {
                                    if ui.button("← Exit Inspection").clicked() {
                                        self.inspection_path.pop();
                                        self.selected_comp_id = None;
                                    }
                                    ui.separator();
                                }

                                // Simulator Stats & Options
                                ui.heading("Options");
                                ui.horizontal(|ui| {
                                    if ui.button("💾 Save").clicked() { self.save_project(); }
                                    if ui.button("📂 Load").clicked() { self.load_project(); }
                                    if ui.button("⚙ Settings").clicked() { self.show_settings = !self.show_settings; }
                                });
                                
                                ui.horizontal(|ui| {
                                    ui.label("Speed:");
                                    ui.add(egui::Slider::new(&mut self.ticks_per_frame, 1..=500).show_value(true));
                                });

                                ui.separator();

                                // Parts Catalog inside drawer
                                if self.inspection_path.is_empty() {
                                    ui.heading("Parts Catalog");
                                    self.draw_catalog_ui(ui);
                                    ui.separator();
                                }

                                // Properties section inside drawer
                                self.draw_properties_ui(ui);

                                ui.add_space(10.0);
                                if ui.button("❌ Close Menu").clicked() {
                                    self.show_menu_mobile = false;
                                }
                            });
                        });
                }
            } else {
                // --- DESKTOP LAYOUT (Big screens) ---
                // 1. Floating Bottom Toolbar for Parts Catalog
                if self.inspection_path.is_empty() {
                    egui::Window::new("Tools")
                        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -20.0))
                        .collapsible(false)
                        .title_bar(false)
                        .resizable(false)
                        .frame(egui::Frame::window(&ctx.style()).inner_margin(8.0))
                        .show(ctx, |ui| {
                            self.draw_catalog_ui(ui);
                        });
                }

                // 2. Control Toolbar (Top Floating Panel)
                egui::Window::new("Controls")
                    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 15.0))
                    .collapsible(false)
                    .title_bar(false)
                    .resizable(false)
                    .frame(egui::Frame::window(&ctx.style()).inner_margin(8.0))
                    .show(ctx, |ui| {
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if !self.inspection_path.is_empty() {
                                    if ui.button("← Exit Inspection").clicked() {
                                        self.inspection_path.pop();
                                        self.selected_comp_id = None;
                                    }
                                    ui.separator();
                                }
                                
                                if ui.button(if self.is_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
                                    self.is_playing = !self.is_playing;
                                }
                                if ui.button("⏭ Step").clicked() {
                                    let _ = self.simulator.propagate_events(50);
                                }

                                ui.separator();
                                ui.label("Speed:");
                                ui.add(egui::Slider::new(&mut self.ticks_per_frame, 1..=500).show_value(false));

                                ui.separator();
                                if ui.button("💾").on_hover_text("Save Project").clicked() { self.save_project(); }
                                if ui.button("📂").on_hover_text("Load Project").clicked() { self.load_project(); }
                                if ui.button("⚙").on_hover_text("Settings").clicked() {
                                    self.show_settings = !self.show_settings;
                                    if self.show_settings {
                                        self.temp_is_fullscreen = self.is_fullscreen;
                                        self.temp_resolution_idx = self.resolution_idx;
                                        self.temp_ui_scale = self.ui_scale;
                                    }
                                }
                            });
                        });
                    });

                // 3. Properties Panel (Right Floating Window)
                egui::Window::new("Properties")
                    .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-15.0, 80.0))
                    .collapsible(true)
                    .resizable(false)
                    .default_width(220.0)
                    .show(ctx, |ui| {
                        self.draw_properties_ui(ui);
                    });
            }

            // Settings Dialog Window
            if self.show_settings {
                let mut show_settings = self.show_settings;
                egui::Window::new("Settings")
                    .open(&mut show_settings)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                    .show(ctx, |ui| {
                        if let Some(timer) = self.resolution_revert_timer {
                            ui.heading("Confirm Resolution Change");
                            ui.add_space(10.0);
                            ui.label(format!("Reverting in {:.1}s...", timer));
                            ui.horizontal(|ui| {
                                if ui.button("Keep Changes").clicked() {
                                    self.resolution_revert_timer = None;
                                }
                                if ui.button("Revert Now").clicked() {
                                    self.resolution_revert_timer = Some(0.0);
                                }
                            });
                        } else {
                            ui.heading("Graphics Settings");
                            let mut temp_fs = self.temp_is_fullscreen;
                            ui.checkbox(&mut temp_fs, "Fullscreen");
                            self.temp_is_fullscreen = temp_fs;

                            ui.label("Resolution:");
                            let resolutions = &[
                                (800, 600, "800 x 600"),
                                (1024, 768, "1024 x 768"),
                                (1280, 720, "1280 x 720 (Default)"),
                                (1600, 900, "1600 x 900"),
                                (1920, 1080, "1920 x 1080"),
                            ];
                            let mut temp_res = self.temp_resolution_idx;
                            egui::ComboBox::from_label("")
                                .selected_text(resolutions[temp_res].2)
                                .show_ui(ui, |ui| {
                                    for (idx, r) in resolutions.iter().enumerate() {
                                        ui.selectable_value(&mut temp_res, idx, r.2);
                                    }
                                });
                            self.temp_resolution_idx = temp_res;

                            let mut temp_scale = self.temp_ui_scale;
                            ui.add(egui::Slider::new(&mut temp_scale, 0.5..=3.0).text("UI Scale"));
                            self.temp_ui_scale = temp_scale;

                            ui.add_space(10.0);
                            if ui.button("Apply Settings").clicked() {
                                self.prev_is_fullscreen = self.is_fullscreen;
                                self.prev_resolution_idx = self.resolution_idx;

                                let size_changed = self.temp_resolution_idx != self.resolution_idx
                                    || self.temp_is_fullscreen != self.is_fullscreen;

                                self.is_fullscreen = self.temp_is_fullscreen;
                                self.resolution_idx = self.temp_resolution_idx;
                                self.ui_scale = self.temp_ui_scale;

                                macroquad::window::set_fullscreen(self.is_fullscreen);
                                let r = resolutions[self.resolution_idx];
                                macroquad::window::request_new_screen_size(r.0 as f32, r.1 as f32);

                                if size_changed {
                                    self.resolution_revert_timer = Some(10.0);
                                }
                            }
                        }
                    });
                self.show_settings = show_settings;
            }
        });
        self.egui_wants_pointer = egui_wants_pointer;
    }
}
