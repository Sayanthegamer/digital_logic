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
            self.ui.ui_scale = self.ui.ui_scale.clamp(0.5, 3.0);
            ctx.set_pixels_per_point(self.ui.ui_scale);

            // Dark elegant theme styling overrides with large touch-friendly targets
            let mut style = (*ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.widgets.active.bg_fill = theme::ACCENT_ACTIVE.egui();
            style.visuals.widgets.hovered.bg_fill = theme::BORDER.egui();
            style.visuals.widgets.noninteractive.bg_fill = theme::BG_PANEL.egui();
            style.visuals.window_fill = theme::BG_PANEL.egui();
            style.visuals.window_stroke = egui::Stroke::new(1.0_f32, theme::BORDER.egui());
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
                AppMode::GlobalLibraryManager => {
                    self.draw_global_library_manager(ctx);
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
                    let mut requested_cancel = false;
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
                            if ui
                                .button(format!("{} Cancel Changes & Return", theme::ICON_CLOSE))
                                .clicked()
                            {
                                requested_cancel = true;
                            }
                        });
                    });

                    if requested_repack {
                        self.save_and_repack_blueprint();
                    } else if requested_cancel {
                        self.cancel_and_return();
                    }
                }

                if is_mobile {
                    self.draw_mobile_editor_ui(ctx);
                } else {
                    self.draw_desktop_editor_ui(ctx);
                }
                
                if self.ui.show_debug_suite {
                    self.draw_debug_suite(ctx);
                }
            } // END OF is_editor BLOCK

            self.draw_settings_dialog(ctx);
            #[cfg(target_os = "android")]
            self.draw_android_file_dialog(ctx);
            self.draw_context_menu(ctx);

            egui_wants_pointer = ctx.wants_pointer_input()
                || ctx.wants_keyboard_input()
                || ctx.is_pointer_over_area();
            self.ui.egui_wants_keyboard = ctx.wants_keyboard_input();
        });
        self.ui.egui_wants_pointer = egui_wants_pointer;
    }

    fn draw_debug_suite(&mut self, ctx: &egui::Context) {
        let mut open = self.ui.show_debug_suite;
        egui::Window::new(format!("{} Debug Suite [F6]", crate::editor::theme::ICON_SETTINGS))
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.heading("Performance Limits");
                let fps = macroquad::time::get_fps();
                ui.label(format!("FPS: {}", fps));
                ui.label(format!("Ticks per Frame: {}", self.engine.ticks_per_frame));
                ui.label(format!("Total Gates: {}", self.engine.simulator.nodes.len()));
                ui.label(format!("Total Active Clocks: {}", self.engine.active_clocks.len()));
                
                ui.separator();
                ui.heading("Threading");
                let mut single = self.ui.debug_single_thread;
                if ui.checkbox(&mut single, "Force Single-Threaded Mode").changed() {
                    self.ui.debug_single_thread = single;
                    self.engine.simulator.set_single_threaded(single);
                }
                
                ui.separator();
                ui.heading("Culling Visualiser");
                ui.checkbox(&mut self.ui.debug_cull_bounds, "Show Culling Bounds (Red=Frustum, Green=Drawn, Gray=Culled)");
                ui.label(format!("Components Drawn: {} / {}", self.ui.drawn_components, self.circuit.components.len()));
                
                ui.separator();
                ui.heading("Software Limits Test");
                ui.horizontal(|ui| {
                    ui.label("Stress Test Recursion Depth:");
                    ui.add(egui::DragValue::new(&mut self.ui.stress_test_size).speed(0.1).range(1..=8));
                });
                ui.horizontal(|ui| {
                    if ui.button("Generate Stress Test!").clicked() {
                        self.generate_stress_test(self.ui.stress_test_size);
                    }
                    if ui.button("Generate Oscillation Crash Test").clicked() {
                        self.generate_oscillation_test();
                    }
                });
                
                ui.separator();
                ui.heading("Logging");
                ui.checkbox(&mut self.ui.debug_continuous_log, "Continuous Logging to debug_suite.log");
                if ui.button("Export Snapshot to Log").clicked() {
                    self.export_debug_log();
                }
            });
        self.ui.show_debug_suite = open;
        
        if self.ui.debug_continuous_log {
            let current_time = macroquad::time::get_time();
            if current_time - self.ui.last_debug_log_time >= 1.0 {
                self.export_debug_log();
                self.ui.last_debug_log_time = current_time;
            }
        }
    }
    
    fn export_debug_log(&self) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("debug_suite.log") {
            let fps = macroquad::time::get_fps();
            let time = macroquad::time::get_time();
            let _ = writeln!(file, "[{:.2}s] FPS: {}, Ticks/Frame: {}, Total Gates: {}, Drawn/Total Comps: {}/{}, Threading: {}", 
                time,
                fps,
                self.engine.ticks_per_frame,
                self.engine.simulator.nodes.len(),
                self.ui.drawn_components,
                self.circuit.components.len(),
                if self.ui.debug_single_thread { "Single" } else { "Multi" }
            );
        }
    }

    fn draw_context_menu(&mut self, ctx: &egui::Context) {
        if self.ui.show_context_menu.is_none() {
            return;
        }

        let target = self.ui.show_context_menu.clone().unwrap();
        let mut keep_open = true;

        let pos = egui::pos2(self.ui.context_menu_pos.0, self.ui.context_menu_pos.1);

        egui::Window::new("Context Menu")
            .fixed_pos(pos)
            .collapsible(false)
            .title_bar(false)
            .resizable(false)
            .auto_sized()
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(25, 28, 32, 240)),
            )
            .show(ctx, |ui| {
                match &target {
                    crate::editor::color_coding::ContextMenuTarget::Component(comp_id) => {
                        let comp_id = *comp_id;
                        ui.label(
                            egui::RichText::new(format!("{} Component Color", theme::ICON_EDIT))
                                .strong()
                                .color(theme::TEXT_PRIMARY.egui()),
                        );
                        ui.add_space(4.0);

                        if ui
                            .color_edit_button_rgba_unmultiplied(&mut self.ui.context_menu_color)
                            .changed()
                        {
                            self.circuit.color_overrides
                                .set_component_color(comp_id, Some(self.ui.context_menu_color));
                        }

                        ui.add_space(4.0);

                        // Preset colors row
                        ui.horizontal(|ui| {
                            let presets: &[[f32; 4]] = &[
                                [1.0, 0.55, 0.15, 1.0],  // Amber
                                [0.00, 0.70, 1.00, 1.0], // Cyan
                                [0.15, 0.85, 0.40, 1.0], // Green
                                [0.90, 0.22, 0.27, 1.0], // Red
                                [0.40, 0.45, 0.85, 1.0], // Indigo
                                [0.85, 0.60, 0.90, 1.0], // Purple
                                [1.0, 0.85, 0.20, 1.0],  // Yellow
                                [0.95, 0.50, 0.60, 1.0], // Pink
                            ];
                            for preset in presets {
                                let c = egui::Color32::from_rgba_unmultiplied(
                                    (preset[0] * 255.0) as u8,
                                    (preset[1] * 255.0) as u8,
                                    (preset[2] * 255.0) as u8,
                                    255,
                                );
                                if ui
                                    .add(egui::Button::new("  ").fill(c))
                                    .clicked()
                                {
                                    self.ui.context_menu_color = *preset;
                                    self.circuit.color_overrides
                                        .set_component_color(comp_id, Some(*preset));
                                }
                            }
                        });

                        ui.add_space(4.0);
                        if ui.button(format!("{} Reset Color", theme::ICON_CLEAR)).clicked() {
                            self.circuit.color_overrides.set_component_color(comp_id, None);
                            keep_open = false;
                        }
                    }
                    crate::editor::color_coding::ContextMenuTarget::Wire(conn) => {
                        let conn = *conn;
                        ui.label(
                            egui::RichText::new(format!("{} Wire Color", theme::ICON_EDIT))
                                .strong()
                                .color(theme::TEXT_PRIMARY.egui()),
                        );
                        ui.add_space(4.0);

                        if ui
                            .color_edit_button_rgba_unmultiplied(&mut self.ui.context_menu_color)
                            .changed()
                        {
                            self.circuit.color_overrides
                                .set_wire_color(&conn, Some(self.ui.context_menu_color));
                        }

                        ui.add_space(4.0);

                        // Preset colors row
                        ui.horizontal(|ui| {
                            let presets: &[[f32; 4]] = &[
                                [1.0, 0.55, 0.15, 1.0],  // Amber
                                [0.00, 0.70, 1.00, 1.0], // Cyan
                                [0.15, 0.85, 0.40, 1.0], // Green
                                [0.90, 0.22, 0.27, 1.0], // Red
                                [0.40, 0.45, 0.85, 1.0], // Indigo
                                [0.85, 0.60, 0.90, 1.0], // Purple
                                [1.0, 0.85, 0.20, 1.0],  // Yellow
                                [0.95, 0.50, 0.60, 1.0], // Pink
                            ];
                            for preset in presets {
                                let c = egui::Color32::from_rgba_unmultiplied(
                                    (preset[0] * 255.0) as u8,
                                    (preset[1] * 255.0) as u8,
                                    (preset[2] * 255.0) as u8,
                                    255,
                                );
                                if ui
                                    .add(egui::Button::new("  ").fill(c))
                                    .clicked()
                                {
                                    self.ui.context_menu_color = *preset;
                                    self.circuit.color_overrides
                                        .set_wire_color(&conn, Some(*preset));
                                }
                            }
                        });

                        ui.add_space(4.0);
                        if ui.button(format!("{} Reset Color", theme::ICON_CLEAR)).clicked() {
                            self.circuit.color_overrides.set_wire_color(&conn, None);
                            keep_open = false;
                        }
                    }
                }

                ui.add_space(4.0);
                if ui.button(format!("{} Close", theme::ICON_CLOSE)).clicked() {
                    keep_open = false;
                }
            });

        if !keep_open {
            self.ui.show_context_menu = None;
        }
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
                        // Use a size-based cap to avoid false "oscillation" errors on large but stable circuits.
                        let max_steps = (self.engine.simulator.nodes.len() * 10).max(1000);
                        match self.engine.simulator.propagate_events(max_steps) {
                            Ok(_) => self.engine.propagation_error = None,
                            Err(e) => self.engine.propagation_error = Some(e),
                        }
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
                                    if ui.button(format!("{} Save", theme::ICON_SAVE)).clicked() {
                                        #[cfg(target_os = "android")]
                                        {
                                            self.ui.show_android_file_dialog = Some(crate::editor::state::AndroidFileDialogMode::Save);
                                            self.ui.android_file_dialog_status.clear();
                                        }
                                        #[cfg(not(target_os = "android"))]
                                        self.save_project();
                                    }
                                    if ui.button(format!("{} Load", theme::ICON_FOLDER)).clicked() {
                                        #[cfg(target_os = "android")]
                                        {
                                            self.ui.show_android_file_dialog = Some(crate::editor::state::AndroidFileDialogMode::Load);
                                            self.ui.android_file_dialog_status.clear();
                                        }
                                        #[cfg(not(target_os = "android"))]
                                        self.load_project();
                                    }
                                    if ui
                                        .button(format!("{} Settings", theme::ICON_SETTINGS))
                                        .clicked()
                                    {
                                        self.ui.show_settings = !self.ui.show_settings;
                                    }
                                });

                                ui.add_space(5.0);
                                if ui.button(format!("{} Auto Arrange", theme::ICON_REFRESH)).clicked() {
                                    self.auto_arrange_components();
                                }

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
                                if ui
                                    .button(format!(
                                        "{} Close Menu",
                                        crate::editor::theme::ICON_CLOSE
                                    ))
                                    .clicked()
                                {
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
                            if ui
                                .button(format!("{} Clear Tool", crate::editor::theme::ICON_CLEAR))
                                .clicked()
                            {
                                self.canvas.selected_tool = None;
                            }
                            ui.separator();

                            if ui
                                .button(format!("{} Undo", crate::editor::theme::ICON_UNDO))
                                .on_hover_text("Undo (Ctrl+Z)")
                                .clicked()
                            {
                                self.undo();
                            }
                            if ui
                                .button(format!("{} Redo", crate::editor::theme::ICON_REDO))
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
                            if ui.button(format!("{} Step", theme::ICON_STOP)).clicked() {
                                // Use a size-based cap to avoid false "oscillation" errors on large but stable circuits.
                                let max_steps = (self.engine.simulator.nodes.len() * 10).max(1000);
                                match self.engine.simulator.propagate_events(max_steps) {
                                    Ok(_) => self.engine.propagation_error = None,
                                    Err(e) => self.engine.propagation_error = Some(e),
                                }
                            }

                            ui.separator();
                            
                            if ui.button(format!("{} Auto Arrange", crate::editor::theme::ICON_REFRESH))
                                .on_hover_text("Automatically arrange components based on connections")
                                .clicked() 
                            {
                                self.auto_arrange_components();
                            }

                            // RECENTER BUTTON CALCULATION
                            if !self.circuit.components.is_empty() {
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
                                for comp in &self.circuit.components {
                                    let p1 = (comp.pos * self.canvas.zoom) + self.canvas.pan;
                                    let p2 = p1
                                        + macroquad::prelude::Vec2::new(comp.width, comp.height)
                                            * self.canvas.zoom;

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
                                        .button(format!(
                                            "{} Recenter",
                                            crate::editor::theme::ICON_RECENTER
                                        ))
                                        .on_hover_text("Focus camera on elements")
                                        .clicked()
                                    {
                                        let mut min_x = f32::MAX;
                                        let mut min_y = f32::MAX;
                                        let mut max_x = f32::MIN;
                                        let mut max_y = f32::MIN;

                                        for comp in &self.circuit.components {
                                            min_x = min_x.min(comp.pos.x);
                                            min_y = min_y.min(comp.pos.y);
                                            max_x = max_x.max(comp.pos.x + comp.width);
                                            max_y = max_y.max(comp.pos.y + comp.height);
                                        }

                                        for ann in &self.circuit.annotations {
                                            min_x = min_x.min(ann.pos.x);
                                            min_y = min_y.min(ann.pos.y);
                                            max_x = max_x.max(ann.pos.x + 150.0);
                                            max_y = max_y.max(ann.pos.y + 20.0);
                                        }

                                        let padding = 100.0;
                                        let w = (max_x - min_x).max(100.0);
                                        let h = (max_y - min_y).max(100.0);

                                        // Target zoom to fit all components into the actual viewport.
                                        let zoom_x = view_w / (w + padding * 2.0);
                                        let zoom_y = view_h / (h + padding * 2.0);
                                        self.canvas.zoom = zoom_x.min(zoom_y).clamp(0.01, 5.0);

                                        // Target pan to center content in the viewport.
                                        let center_x = (min_x + max_x) / 2.0;
                                        let center_y = (min_y + max_y) / 2.0;
                                        self.canvas.pan = macroquad::prelude::Vec2::new(
                                            view_min_x + view_w / 2.0,
                                            view_min_y + view_h / 2.0,
                                        ) - macroquad::prelude::Vec2::new(
                                            center_x, center_y,
                                        ) * self.canvas.zoom;
                                    }
                                    ui.separator();
                                }
                            }

                            ui.label("Speed:");
                            ui.add(
                                egui::Slider::new(&mut self.engine.ticks_per_frame, 1..=500)
                                    .show_value(false),
                            );

                            ui.separator();
                            if ui
                                .button(theme::ICON_SAVE)
                                .on_hover_text("Save Project")
                                .clicked()
                            {
                                #[cfg(target_os = "android")]
                                {
                                    self.ui.show_android_file_dialog = Some(crate::editor::state::AndroidFileDialogMode::Save);
                                    self.ui.android_file_dialog_status.clear();
                                }
                                #[cfg(not(target_os = "android"))]
                                self.save_project();
                            }
                            if ui
                                .button(theme::ICON_FOLDER)
                                .on_hover_text("Load Project")
                                .clicked()
                            {
                                #[cfg(target_os = "android")]
                                {
                                    self.ui.show_android_file_dialog = Some(crate::editor::state::AndroidFileDialogMode::Load);
                                    self.ui.android_file_dialog_status.clear();
                                }
                                #[cfg(not(target_os = "android"))]
                                self.load_project();
                            }
                            if ui
                                .button(theme::ICON_IMAGE)
                                .on_hover_text("Export SVG Picture")
                                .clicked()
                            {
                                self.export_to_svg();
                            }
                            if ui
                                .button(theme::ICON_SETTINGS)
                                .on_hover_text("Settings")
                                .clicked()
                            {
                                self.ui.show_settings = !self.ui.show_settings;
                                if self.ui.show_settings {
                                    self.ui.temp_is_fullscreen = self.ui.is_fullscreen;
                                    self.ui.temp_resolution_idx = self.ui.resolution_idx;
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

        // Cache the actual canvas viewport (remaining area) in logical pixels.
        let r = ctx.available_rect();
        self.ui.canvas_viewport.replace((
            r.min.x,
            r.min.y,
            r.width(),
            r.height(),
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

    #[cfg(target_os = "android")]
    fn draw_android_file_dialog(&mut self, ctx: &egui::Context) {
        if let Some(mode) = self.ui.show_android_file_dialog {
            let mut open = true;
            let title = match mode {
                crate::editor::state::AndroidFileDialogMode::Save => "Save Project",
                crate::editor::state::AndroidFileDialogMode::Load => "Load Project",
            };

            egui::Window::new(title)
                .open(&mut open)
                .resizable(true)
                .default_width(320.0)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .show(ctx, |ui| {
                    if !self.ui.android_file_dialog_status.is_empty() {
                        ui.colored_label(egui::Color32::LIGHT_GREEN, &self.ui.android_file_dialog_status);
                        ui.add_space(5.0);
                    }

                    let dir_res = super::persistence::get_android_external_files_dir()
                        .or_else(|_| super::persistence::get_android_internal_files_dir());
                    
                    let mut files = Vec::new();
                    if let Ok(dir) = dir_res {
                        if let Ok(entries) = std::fs::read_dir(dir) {
                            for entry in entries.flatten() {
                                if let Ok(metadata) = entry.metadata() {
                                    if metadata.is_file() {
                                        let name = entry.file_name().to_string_lossy().into_owned();
                                        if name.ends_with(".logic") || name.ends_with(".json") {
                                            files.push(name);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    files.sort();

                    if mode == crate::editor::state::AndroidFileDialogMode::Save {
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.ui.android_file_dialog_input);
                        });
                        
                        ui.add_space(10.0);
                        if ui.button("Save File").clicked() {
                            let mut filename = self.ui.android_file_dialog_input.trim().to_string();
                            if filename.is_empty() {
                                filename = "project_save.logic".to_string();
                            }
                            if !filename.ends_with(".logic") && !filename.ends_with(".json") {
                                filename.push_str(".logic");
                            }
                            
                            if let Ok(dir) = super::persistence::get_android_external_files_dir()
                                .or_else(|_| super::persistence::get_android_internal_files_dir()) 
                            {
                                let mut path = dir;
                                path.push(&filename);
                                if let Err(err) = self.save_to_path(path) {
                                    self.ui.android_file_dialog_status = format!("Save failed: {err}");
                                } else {
                                    self.ui.android_file_dialog_status = format!("Saved as {filename}");
                                }
                            } else {
                                self.ui.android_file_dialog_status = "Failed to resolve save directory".to_string();
                            }
                        }
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.label("Existing Files (Click to select/load):");
                    
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        if files.is_empty() {
                            ui.label("(No projects found)");
                        } else {
                            for file in files {
                                let selected = self.ui.android_file_dialog_input == file;
                                if ui.selectable_label(selected, &file).clicked() {
                                    self.ui.android_file_dialog_input = file.clone();
                                    if mode == crate::editor::state::AndroidFileDialogMode::Load {
                                        if let Ok(dir) = super::persistence::get_android_external_files_dir()
                                            .or_else(|_| super::persistence::get_android_internal_files_dir()) 
                                        {
                                            let mut path = dir;
                                            path.push(&file);
                                            if self.load_from_path(&path) {
                                                self.ui.android_file_dialog_status = format!("Loaded {file}");
                                            } else {
                                                self.ui.android_file_dialog_status = format!("Failed to load {file}");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
                });

            if !open {
                self.ui.show_android_file_dialog = None;
            }
        }
    }
}
