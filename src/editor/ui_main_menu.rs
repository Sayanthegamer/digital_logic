#![allow(clippy::collapsible_if)]
use super::Editor;
use super::state::AppMode;
use crate::editor::theme;
use crate::engine::ComponentType;

impl Editor {
    pub(crate) fn draw_main_menu(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();

        egui::Area::new(egui::Id::new("main_menu_area"))
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                ui.set_width(screen_rect.width());
                ui.set_height(screen_rect.height());

                // Semi-transparent background over the canvas grid
                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(220));

                ui.vertical_centered(|ui| {
                    ui.add_space(screen_rect.height() * 0.2);

                    // Title
                    ui.heading(
                        egui::RichText::new("Digital Logic Simulator")
                            .size(64.0)
                            .color(theme::TEXT_PRIMARY.egui())
                            .strong(),
                    );

                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("Design, simulate, and build custom chips")
                            .size(24.0)
                            .color(theme::TEXT_SECONDARY.egui()),
                    );

                    ui.add_space(screen_rect.height() * 0.15);

                    // Menu Buttons
                    let button_width = 300.0;
                    let button_height = 50.0;
                    let font_size = 24.0;

                    let mut clicked_new = false;
                    let mut clicked_open = false;
                    let mut clicked_manage = false;
                    let mut clicked_settings = false;
                    let mut clicked_credits = false;

                    let menu_btn = |ui: &mut egui::Ui, text: &str, icon: &str| -> bool {
                        ui.add_sized(
                            [button_width, button_height],
                            egui::Button::new(
                                egui::RichText::new(format!("{}  {}", icon, text))
                                    .size(font_size)
                                    .color(theme::TEXT_PRIMARY.egui()),
                            )
                            .fill(theme::BG_CANVAS.egui()),
                        )
                        .clicked()
                    };

                    if menu_btn(ui, "New Project", theme::ICON_ADD) {
                        clicked_new = true;
                    }

                    ui.add_space(15.0);

                    if menu_btn(ui, "Open Project", theme::ICON_FOLDER) {
                        clicked_open = true;
                    }

                    ui.add_space(15.0);

                    if menu_btn(ui, "Manage Chips", theme::ICON_EDIT) {
                        clicked_manage = true;
                    }

                    ui.add_space(15.0);

                    if menu_btn(ui, "Settings", theme::ICON_SETTINGS) {
                        clicked_settings = true;
                    }

                    ui.add_space(15.0);

                    if menu_btn(ui, "Credits", theme::ICON_INFO) {
                        clicked_credits = true;
                    }

                    if clicked_new {
                        *self = Editor::new();
                        self.ui.mode = AppMode::Editor;
                    } else if clicked_open {
                        if self.load_project() {
                            self.ui.mode = AppMode::Editor;
                        }
                    } else if clicked_manage {
                        self.ui.mode = AppMode::ManageChips;
                    } else if clicked_settings {
                        self.ui.show_settings = true;
                        self.ui.temp_is_fullscreen = self.ui.is_fullscreen;
                        self.ui.temp_resolution_idx = self.ui.resolution_idx;
                        self.ui.temp_ui_scale = self.ui.ui_scale;
                    } else if clicked_credits {
                        self.ui.mode = AppMode::Credits;
                    }
                });
            });
    }

    pub(crate) fn draw_credits(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();

        egui::Area::new(egui::Id::new("credits_area"))
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                ui.set_width(screen_rect.width());
                ui.set_height(screen_rect.height());

                ui.painter().rect_filled(
                    screen_rect,
                    0.0,
                    egui::Color32::from_black_alpha(220),
                );

                ui.vertical_centered(|ui| {
                    ui.add_space(screen_rect.height() * 0.1);

                    ui.heading(
                        egui::RichText::new("Credits")
                            .size(48.0)
                            .color(theme::ACCENT_PRIMARY.egui())
                            .strong()
                    );

                    ui.add_space(40.0);
                    let credit = |ui: &mut egui::Ui, title: &str, name: &str| {
                        ui.label(
                            egui::RichText::new(title)
                                .size(20.0)
                                .color(theme::TEXT_SECONDARY.egui())
                        );
                        ui.label(
                            egui::RichText::new(name)
                                .size(28.0)
                                .color(theme::TEXT_PRIMARY.egui())
                                .strong()
                        );
                        ui.add_space(20.0);
                    };

                    credit(ui, "Creator & Founder", "Sayan Das");
                    ui.label(egui::RichText::new("(i am everything, i did everything by myself so i am the founder of the whole thing (for now))").size(14.0).color(theme::TEXT_SECONDARY.egui()));
                    ui.add_space(30.0);

                    credit(ui, "AI Pair Programming", "Google GenAI");
                    credit(ui, "AI Agent", "Jules");
                    credit(ui, "Code Review", "Qodo");

                    ui.add_space(screen_rect.height() * 0.1);

                    if ui.add_sized(
                        [200.0, 50.0],
                        egui::Button::new(
                            egui::RichText::new("Back")
                                .size(24.0)
                                .color(theme::TEXT_PRIMARY.egui())
                        ).fill(theme::BG_CANVAS.egui())
                    ).clicked() {
                        self.ui.mode = AppMode::MainMenu;
                    }
                });
            });
    }

    pub(crate) fn draw_manage_chips(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();

        egui::Area::new(egui::Id::new("manage_chips_area"))
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                ui.set_width(screen_rect.width());
                ui.set_height(screen_rect.height());

                ui.painter()
                    .rect_filled(screen_rect, 0.0, egui::Color32::from_black_alpha(220));

                ui.vertical_centered(|ui| {
                    ui.add_space(screen_rect.height() * 0.1);

                    ui.heading(
                        egui::RichText::new("Manage Saved Chips")
                            .size(48.0)
                            .color(theme::ACCENT_PRIMARY.egui())
                            .strong(),
                    );

                    ui.add_space(40.0);

                    // Add a scroll area for chips
                    egui::ScrollArea::vertical()
                        .max_height(screen_rect.height() * 0.5)
                        .show(ui, |ui| {
                            let mut to_delete = None;

                            for (idx, bp) in self.engine.library.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.add_space(screen_rect.width() * 0.3);

                                    ui.label(
                                        egui::RichText::new(&bp.name)
                                            .size(20.0)
                                            .color(theme::TEXT_PRIMARY.egui()),
                                    );

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(screen_rect.width() * 0.3);
                                            if ui
                                                .button(
                                                    egui::RichText::new(theme::ICON_DELETE)
                                                        .color(theme::ACCENT_ERROR.egui()),
                                                )
                                                .clicked()
                                            {
                                                to_delete = Some(idx);
                                            }
                                        },
                                    );
                                });
                                ui.add_space(10.0);
                            }

                            if let Some(idx) = to_delete {
                                self.engine.library.remove(idx);
                                // Shift all SubChip indices down by 1 if they are > idx
                                for c in &mut self.components {
                                    if let ComponentType::SubChip(ref mut i) = c.comp_type {
                                        if *i > idx {
                                            *i -= 1;
                                        }
                                    }
                                }
                                // Drop any components of this exact type
                                let to_remove: Vec<_> = self
                                    .components
                                    .iter()
                                    .filter(|c| c.comp_type == ComponentType::SubChip(idx))
                                    .map(|c| c.id)
                                    .collect();
                                self.components
                                    .retain(|c| c.comp_type != ComponentType::SubChip(idx));
                                self.connections.retain(|w| {
                                    !to_remove.contains(&w.src_comp_id)
                                        && !to_remove.contains(&w.tgt_comp_id)
                                });
                                self.compile();
                            }

                            if self.engine.library.is_empty() {
                                ui.label(
                                    egui::RichText::new("No custom chips saved yet.")
                                        .size(20.0)
                                        .color(theme::TEXT_SECONDARY.egui()),
                                );
                            }
                        });

                    ui.add_space(screen_rect.height() * 0.1);

                    if ui
                        .add_sized(
                            [200.0, 50.0],
                            egui::Button::new(
                                egui::RichText::new("Back")
                                    .size(24.0)
                                    .color(theme::TEXT_PRIMARY.egui()),
                            )
                            .fill(theme::BG_CANVAS.egui()),
                        )
                        .clicked()
                    {
                        self.ui.mode = AppMode::MainMenu;
                    }
                });
            });
    }
}
