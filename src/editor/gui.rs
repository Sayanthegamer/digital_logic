use super::Editor;
use crate::editor::theme;

pub fn setup_egui() {
    egui_macroquad::ui(|ctx| {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "material_icons".to_owned(),
            egui::FontData::from_static(include_bytes!("../../MaterialIcons-Regular.ttf")).into(),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(1, "material_icons".to_owned());
        fonts
            .families
            .get_mut(&egui::FontFamily::Monospace)
            .unwrap()
            .push("material_icons".to_owned());
        ctx.set_fonts(fonts);
    });
}

impl Editor {
    pub fn draw_gui(&mut self) {
        let mut egui_wants_pointer = false;
        let is_mobile = macroquad::window::screen_width() < 720.0;

        egui_macroquad::ui(|ctx| {
            ctx.set_pixels_per_point(self.ui.ui_scale);

            // Dark elegant theme styling overrides with large touch-friendly targets
            let mut style = (*ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.widgets.active.bg_fill = theme::ACCENT_ACTIVE.egui();
            style.visuals.widgets.hovered.bg_fill = theme::BORDER.egui();
            style.visuals.widgets.noninteractive.bg_fill = theme::BG_PANEL.egui();
            style.visuals.window_fill = theme::BG_PANEL.egui();
            style.visuals.window_stroke = egui::Stroke::new(1.0, theme::BORDER.egui());
            style.visuals.window_corner_radius = egui::CornerRadius::same(12);
            style.spacing.button_padding = egui::vec2(14.0, 10.0);
            style.spacing.item_spacing = egui::vec2(12.0, 12.0);
            ctx.set_style(style);

            use crate::editor::state::AppMode;
            let is_editor = self.ui.mode == AppMode::Editor;

            match self.ui.mode {
                AppMode::MainMenu => {
                    self.draw_main_menu(ctx);
                }
                AppMode::Credits => {
                    self.draw_credits(ctx);
                }
                AppMode::ManageChips => {
                    self.draw_manage_chips(ctx);
                }
                AppMode::Editor => {}
            }

            if is_editor {
                // Error panel if simulation has oscillated/errored
                if let Some(ref err) = self.engine.propagation_error {
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

                // Edit Blueprint Panel
                if let crate::editor::state::EditingTarget::LibraryChip(bp_idx) =
                    self.canvas.editing_target
                {
                    let mut requested_repack = false;
                    egui::TopBottomPanel::top("editing_chip_panel").show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            let bp_name = self
                                .engine
                                .library
                                .get(bp_idx)
                                .map(|bp| bp.name.clone())
                                .unwrap_or_default();
                            ui.colored_label(
                                egui::Color32::from_rgb(100, 255, 100),
                                format!("✏️ Editing Blueprint: {}", bp_name),
                            );
                            ui.add_space(20.0);
                            if ui
                                .button(format!("{} Save Changes & Return", theme::ICON_SAVE))
                                .clicked()
                            {
                                requested_repack = true;
                            }
                        });
                    });

                    if requested_repack {
                        self.save_and_repack_blueprint();
                    }
                }

                if is_mobile {
                    self.draw_mobile_editor_ui(ctx);
                } else {
                    self.draw_desktop_editor_ui(ctx);
                }
            } // END OF is_editor BLOCK

            self.draw_settings_dialog(ctx);

            egui_wants_pointer = ctx.wants_pointer_input()
                || ctx.wants_keyboard_input()
                || ctx.is_pointer_over_area();
        });
        self.ui.egui_wants_pointer = egui_wants_pointer;
    }

    fn draw_mobile_editor_ui(&mut self, ctx: &egui::Context) {
        // 1. Tiny Transparent FAB for Menu & Play/Pause at the top
        egui::Window::new("Mobile FAB")
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(15.0, 15.0))
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(18, 20, 22, 180)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(format!("{} Menu", theme::ICON_SETTINGS))
                        .clicked()
                    {
                        self.ui.show_menu_mobile = !self.ui.show_menu_mobile;
                    }
                    ui.separator();
                    if ui
                        .button(if self.engine.is_playing {
                            theme::ICON_PAUSE
                        } else {
                            theme::ICON_PLAY
                        })
                        .clicked()
                    {
                        self.engine.is_playing = !self.engine.is_playing;
                    }
                    if ui.button(theme::ICON_STOP).clicked() {
                        let _ = self.engine.simulator.propagate_events(50);
                    }
                });
            });

        // 2. Full Control Drawer (overlay)
        if self.ui.show_menu_mobile {
            egui::Window::new("Simulator Drawer")
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .title_bar(true)
                .resizable(false)
                .default_width(320.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(macroquad::window::screen_height() * 0.7)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                if !self.canvas.inspection_path.is_empty() {
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
                                    ui.separator();
                                }

                                // Simulator Stats & Options
                                ui.heading("Options");
                                ui.horizontal(|ui| {
                                    if ui
                                        .button(format!("{} Save", theme::ICON_SAVE))
                                        .clicked()
                                    {
                                        self.save_project();
                                    }
                                    if ui
                                        .button(format!("{} Load", theme::ICON_FOLDER))
                                        .clicked()
                                    {
                                        self.load_project();
                                    }
                                    if ui
                                        .button(format!(
                                            "{} Settings",
                                            theme::ICON_SETTINGS
                                        ))
                                        .clicked()
                                    {
                                        self.ui.show_settings = !self.ui.show_settings;
                                    }
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Speed:");
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.engine.ticks_per_frame,
                                            1..=500,
                                        )
                                        .show_value(true),
                                    );
                                });

                                ui.separator();

                                // Parts Catalog inside drawer
                                if self.canvas.inspection_path.is_empty() {
                                    ui.heading("Parts Catalog");
                                    self.draw_catalog_ui(ui);
                                    ui.separator();
                                }

                                // Properties section inside drawer
                                self.draw_properties_ui(ui);

                                ui.add_space(10.0);
                                if ui.button("❌ Close Menu").clicked() {
                                    self.ui.show_menu_mobile = false;
                                }
                            });
                        });
                });
        }
    }

    fn draw_desktop_editor_ui(&mut self, ctx: &egui::Context) {
        // 1. Top Controls Panel
        egui::TopBottomPanel::top("controls_panel")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(8.0))
            .show(ctx, |ui| {
                let mut scroll_by = self.ui.controls_scroll_request.take();

                // Handle vertical mouse wheel as horizontal scroll
                let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                if scroll_delta.y != 0.0 {
                    scroll_by = Some(scroll_delta.y * 3.0);
                }

                ui.horizontal(|ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        if let Some(delta) = scroll_by {
                            ui.scroll_with_delta(egui::vec2(delta, 0.0));
                        }
                        ui.horizontal(|ui| {
                            if ui.button("❌ Clear Tool").clicked() {
                                self.canvas.selected_tool = None;
                            }
                            ui.separator();

                            if ui
                                .button("↩ Undo")
                                .on_hover_text("Undo (Ctrl+Z)")
                                .clicked()
                            {
                                self.undo();
                            }
                            if ui
                                .button("↪ Redo")
                                .on_hover_text("Redo (Ctrl+Y)")
                                .clicked()
                            {
                                self.redo();
                            }
                            ui.separator();
                            if !self.canvas.inspection_path.is_empty() {
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
                                ui.separator();
                            }

                            if ui
                                .button(if self.engine.is_playing {
                                    format!("{} Pause", theme::ICON_PAUSE)
                                } else {
                                    format!("{} Play", theme::ICON_PLAY)
                                })
                                .clicked()
                            {
                                self.engine.is_playing = !self.engine.is_playing;
                            }
                            if ui.button(format!("{} Step", theme::ICON_STOP)).clicked()
                            {
                                let _ = self.engine.simulator.propagate_events(50);
                            }

                            ui.separator();

                            // RECENTER BUTTON CALCULATION
                            if !self.components.is_empty() {
                                let (screen_w, screen_h) = (
                                    macroquad::prelude::screen_width(),
                                    macroquad::prelude::screen_height(),
                                );

                                // Use the real canvas viewport (remaining rect after egui panels).
                                let (vx, vy, vw, vh) = self
                                    .ui
                                    .canvas_viewport
                                    .unwrap_or((0.0, 0.0, screen_w, screen_h));
                                let view_w = vw.max(50.0);
                                let view_h = vh.max(50.0);
                                let view_min_x = vx;
                                let view_min_y = vy;
                                let view_max_x = vx + view_w;
                                let view_max_y = vy + view_h;

                                let mut any_outside_viewport = false;
                                for comp in &self.components {
                                    let p1 =
                                        (comp.pos * self.canvas.zoom) + self.canvas.pan;
                                    let p2 = p1
                                        + macroquad::prelude::Vec2::new(
                                            comp.width,
                                            comp.height,
                                        ) * self.canvas.zoom;

                                    // Show recenter if any element is even partially out of view.
                                    if p1.x < view_min_x
                                        || p2.x > view_max_x
                                        || p1.y < view_min_y
                                        || p2.y > view_max_y
                                    {
                                        any_outside_viewport = true;
                                        break;
                                    }
                                }

                                if any_outside_viewport {
                                    if ui
                                        .button("🎯 Recenter")
                                        .on_hover_text("Focus camera on elements")
                                        .clicked()
                                    {
                                        let mut min_x = f32::MAX;
                                        let mut min_y = f32::MAX;
                                        let mut max_x = f32::MIN;
                                        let mut max_y = f32::MIN;

                                        for comp in &self.components {
                                            min_x = min_x.min(comp.pos.x);
                                            min_y = min_y.min(comp.pos.y);
                                            max_x = max_x.max(comp.pos.x + comp.width);
                                            max_y = max_y.max(comp.pos.y + comp.height);
                                        }

                                        let padding = 100.0;
                                        let w = (max_x - min_x).max(100.0);
                                        let h = (max_y - min_y).max(100.0);

                                        // Target zoom to fit all components into the actual viewport.
                                        let zoom_x = view_w / (w + padding * 2.0);
                                        let zoom_y = view_h / (h + padding * 2.0);
                                        self.canvas.zoom =
                                            zoom_x.min(zoom_y).clamp(0.1, 5.0);

                                        // Target pan to center content in the viewport.
                                        let center_x = min_x + w / 2.0;
                                        let center_y = min_y + h / 2.0;
                                        self.canvas.pan = macroquad::prelude::Vec2::new(
                                            view_min_x + view_w / 2.0,
                                            view_min_y + view_h / 2.0,
                                        )
                                            - macroquad::prelude::Vec2::new(
                                                center_x, center_y,
                                            ) * self.canvas.zoom;
                                    }
                                    ui.separator();
                                }
                            }

                            ui.label("Speed:");
                            ui.add(
                                egui::Slider::new(
                                    &mut self.engine.ticks_per_frame,
                                    1..=500,
                                )
                                .show_value(false),
                            );

                            ui.separator();
                            if ui
                                .button(theme::ICON_SAVE)
                                .on_hover_text("Save Project")
                                .clicked()
                            {
                                self.save_project();
                            }
                            if ui
                                .button(theme::ICON_FOLDER)
                                .on_hover_text("Load Project")
                                .clicked()
                            {
                                self.load_project();
                            }
                            if ui
                                .button(theme::ICON_SETTINGS)
                                .on_hover_text("Settings")
                                .clicked()
                            {
                                self.ui.show_settings = !self.ui.show_settings;
                                if self.ui.show_settings {
                                    self.ui.temp_is_fullscreen = self.ui.is_fullscreen;
                                    self.ui.temp_resolution_idx =
                                        self.ui.resolution_idx;
                                    self.ui.temp_ui_scale = self.ui.ui_scale;
                                }
                            }
                        });
                    });
                });
            });

        // 2. Left Catalog Panel
        if self.canvas.inspection_path.is_empty() {
            egui::SidePanel::left("catalog_panel")
                .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(12.0))
                .min_width(180.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        self.draw_catalog_ui(ui);
                    });
                });
        }

        // 3. Right Properties Panel
        egui::SidePanel::right("properties_panel")
            .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(12.0))
            .min_width(240.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.draw_properties_ui(ui);
                });
            });

        // Cache the actual canvas viewport (remaining area) in screen pixels.
        let r = ctx.available_rect();
        let ppp = ctx.pixels_per_point();
        self.ui.canvas_viewport.replace((
            r.min.x * ppp,
            r.min.y * ppp,
            r.width() * ppp,
            r.height() * ppp,
        ));
    }

    fn draw_settings_dialog(&mut self, ctx: &egui::Context) {
        if self.ui.show_settings {
            let mut show_settings = self.ui.show_settings;
            egui::Window::new("Settings")
                .open(&mut show_settings)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    if let Some(timer) = self.ui.resolution_revert_timer {
                        ui.heading("Confirm Resolution Change");
                        ui.add_space(10.0);
                        ui.label(format!("Reverting in {:.1}s...", timer));
                        ui.horizontal(|ui| {
                            if ui.button("Keep Changes").clicked() {
                                self.ui.resolution_revert_timer = None;
                            }
                            if ui.button("Revert Now").clicked() {
                                self.ui.resolution_revert_timer = Some(0.0);
                            }
                        });
                    } else {
                        ui.heading("Graphics Settings");
                        let mut temp_fs = self.ui.temp_is_fullscreen;
                        ui.checkbox(&mut temp_fs, "Fullscreen");
                        self.ui.temp_is_fullscreen = temp_fs;

                        ui.label("Resolution:");
                        let resolutions = &[
                            (800, 600, "800 x 600"),
                            (1024, 768, "1024 x 768"),
                            (1280, 720, "1280 x 720 (Default)"),
                            (1600, 900, "1600 x 900"),
                            (1920, 1080, "1920 x 1080"),
                        ];
                        let mut temp_res = self.ui.temp_resolution_idx;
                        egui::ComboBox::from_label("")
                            .selected_text(resolutions[temp_res].2)
                            .show_ui(ui, |ui| {
                                for (idx, r) in resolutions.iter().enumerate() {
                                    ui.selectable_value(&mut temp_res, idx, r.2);
                                }
                            });
                        self.ui.temp_resolution_idx = temp_res;

                        let mut temp_scale = self.ui.temp_ui_scale;
                        ui.add(egui::Slider::new(&mut temp_scale, 0.5..=3.0).text("UI Scale"));
                        self.ui.temp_ui_scale = temp_scale;

                        ui.add_space(10.0);
                        if ui.button("Apply Settings").clicked() {
                            self.ui.prev_is_fullscreen = self.ui.is_fullscreen;
                            self.ui.prev_resolution_idx = self.ui.resolution_idx;

                            let size_changed = self.ui.temp_resolution_idx
                                != self.ui.resolution_idx
                                || self.ui.temp_is_fullscreen != self.ui.is_fullscreen;

                            self.ui.is_fullscreen = self.ui.temp_is_fullscreen;
                            self.ui.resolution_idx = self.ui.temp_resolution_idx;
                            self.ui.ui_scale = self.ui.temp_ui_scale;

                            macroquad::window::set_fullscreen(self.ui.is_fullscreen);
                            let r = resolutions[self.ui.resolution_idx];
                            macroquad::window::request_new_screen_size(r.0 as f32, r.1 as f32);

                            if size_changed {
                                self.ui.resolution_revert_timer = Some(10.0);
                            }
                        }
                    }
                });
            self.ui.show_settings = show_settings;
        }
    }
}
