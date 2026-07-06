#![allow(clippy::collapsible_if)]
use super::Editor;
use super::state::AppMode;
use crate::editor::theme;
use crate::editor::global_library;

impl Editor {
    pub(crate) fn draw_global_library_manager(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();
        let margin = screen_rect.width() * 0.05;
        let panel_height = screen_rect.height() * 0.65;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::from_rgba_unmultiplied(25, 28, 32, 230)))
            .show(ctx, |ui| {
                // Title
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.heading(
                        egui::RichText::new(format!("{} Global Chip Library", theme::ICON_FOLDER))
                            .size(36.0)
                            .color(theme::ACCENT_PRIMARY.egui())
                            .strong(),
                    );
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new("Chips here are available across all projects")
                            .size(16.0)
                            .color(theme::TEXT_SECONDARY.egui()),
                    );
                    ui.add_space(15.0);
                });

                // Main content area — two columns
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.vertical(|ui| {
                        ui.set_width(screen_rect.width() - margin * 2.0);

                        ui.horizontal(|ui| {
                            // Left panel: Folders
                            let left_w = (screen_rect.width() - margin * 2.0) * 0.35;
                            ui.vertical(|ui| {
                                ui.set_width(left_w);
                                ui.set_max_height(panel_height);

                                ui.heading(
                                    egui::RichText::new(format!("{} Folders", theme::ICON_FOLDER))
                                        .size(20.0)
                                        .color(theme::TEXT_PRIMARY.egui()),
                                );
                                ui.add_space(5.0);

                                // New Folder button
                                if ui
                                    .button(format!("{} New Folder", theme::ICON_ADD))
                                    .clicked()
                                {
                                    self.global_library.folders.push(global_library::ChipFolder {
                                        name: format!("Folder {}", self.global_library.folders.len() + 1),
                                        color: None,
                                        chips: Vec::new(),
                                    });
                                    global_library::save_global_library(&self.global_library);
                                }

                                ui.add_space(5.0);

                                // "Ungrouped" pseudo-folder
                                egui::ScrollArea::vertical()
                                    .id_salt("global_lib_folders")
                                    .max_height(panel_height - 80.0)
                                    .show(ui, |ui| {
                                        let is_ungrouped_selected = self.ui.global_lib_selected_folder.is_none();
                                        if ui
                                            .selectable_label(
                                                is_ungrouped_selected,
                                                format!("{} Ungrouped ({})", theme::ICON_FOLDER, self.global_library.ungrouped.len()),
                                            )
                                            .clicked()
                                        {
                                            self.ui.global_lib_selected_folder = None;
                                        }

                                        ui.add_space(3.0);

                                        let mut folder_to_delete = None;
                                        for (fi, folder) in self.global_library.folders.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                // Color swatch
                                                if let Some(col) = folder.color {
                                                    let c = egui::Color32::from_rgba_unmultiplied(
                                                        (col[0] * 255.0) as u8,
                                                        (col[1] * 255.0) as u8,
                                                        (col[2] * 255.0) as u8,
                                                        (col[3] * 255.0) as u8,
                                                    );
                                                    let (rect, _) = ui.allocate_exact_size(
                                                        egui::vec2(12.0, 12.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    ui.painter().rect_filled(rect, 3.0, c);
                                                }

                                                let selected = self.ui.global_lib_selected_folder == Some(fi);
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        format!("{} ({} chips)", folder.name, folder.chips.len()),
                                                    )
                                                    .clicked()
                                                {
                                                    self.ui.global_lib_selected_folder = Some(fi);
                                                }

                                                if ui
                                                    .small_button(
                                                        egui::RichText::new(theme::ICON_DELETE)
                                                            .color(theme::ACCENT_ERROR.egui()),
                                                    )
                                                    .on_hover_text("Delete folder (chips move to Ungrouped)")
                                                    .clicked()
                                                {
                                                    folder_to_delete = Some(fi);
                                                }
                                            });
                                        }

                                        if let Some(fi) = folder_to_delete {
                                            // Move chips to ungrouped before deleting
                                            let chips = self.global_library.folders[fi].chips.clone();
                                            self.global_library.ungrouped.extend(chips);
                                            self.global_library.folders.remove(fi);
                                            if self.ui.global_lib_selected_folder == Some(fi) {
                                                self.ui.global_lib_selected_folder = None;
                                            }
                                            self.sync_engine_library();
                                            global_library::save_global_library(&self.global_library);
                                        }
                                    });
                            });

                            ui.separator();

                            // Right panel: Chip list for selected folder
                            let right_w = (screen_rect.width() - margin * 2.0) * 0.60;
                            ui.vertical(|ui| {
                                ui.set_width(right_w);
                                ui.set_max_height(panel_height);

                                let (folder_name, chips_ref_len) = match self.ui.global_lib_selected_folder {
                                    Some(fi) if fi < self.global_library.folders.len() => {
                                        (self.global_library.folders[fi].name.clone(), self.global_library.folders[fi].chips.len())
                                    }
                                    _ => ("Ungrouped".to_string(), self.global_library.ungrouped.len()),
                                };

                                ui.heading(
                                    egui::RichText::new(format!("{} — {} chips", folder_name, chips_ref_len))
                                        .size(20.0)
                                        .color(theme::TEXT_PRIMARY.egui()),
                                );
                                ui.add_space(5.0);

                                // Folder rename (if a folder is selected)
                                if let Some(fi) = self.ui.global_lib_selected_folder {
                                    if fi < self.global_library.folders.len() {
                                        ui.horizontal(|ui| {
                                            ui.label("Rename:");
                                            let mut name = self.global_library.folders[fi].name.clone();
                                            if ui.text_edit_singleline(&mut name).changed() {
                                                self.global_library.folders[fi].name = name;
                                                global_library::save_global_library(&self.global_library);
                                            }
                                        });

                                        // Folder colour
                                        ui.horizontal(|ui| {
                                            ui.label("Folder Color:");
                                            let mut col = self.global_library.folders[fi]
                                                .color
                                                .unwrap_or([0.4, 0.45, 0.85, 1.0]);
                                            if ui.color_edit_button_rgba_unmultiplied(&mut col).changed() {
                                                self.global_library.folders[fi].color = Some(col);
                                                global_library::save_global_library(&self.global_library);
                                            }
                                            if ui.small_button("Reset").clicked() {
                                                self.global_library.folders[fi].color = None;
                                                global_library::save_global_library(&self.global_library);
                                            }
                                        });

                                        ui.add_space(5.0);
                                    }
                                }

                                egui::ScrollArea::vertical()
                                    .id_salt("global_lib_chips")
                                    .max_height(panel_height - 120.0)
                                    .show(ui, |ui| {
                                        let chips: Vec<(String, usize, usize)> = match self.ui.global_lib_selected_folder {
                                            Some(fi) if fi < self.global_library.folders.len() => {
                                                self.global_library.folders[fi]
                                                    .chips
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(_ci, bp)| (bp.name.clone(), bp.inputs, bp.outputs))
                                                    .collect()
                                            }
                                            _ => {
                                                self.global_library
                                                    .ungrouped
                                                    .iter()
                                                    .enumerate()
                                                    .map(|(_ci, bp)| (bp.name.clone(), bp.inputs, bp.outputs))
                                                    .collect()
                                            }
                                        };

                                        if chips.is_empty() {
                                            ui.label(
                                                egui::RichText::new("No chips in this group")
                                                    .color(theme::TEXT_SECONDARY.egui()),
                                            );
                                        }

                                        let mut chip_to_delete = None;
                                        let mut chip_to_move = None;

                                        for (ci, (name, inputs, outputs)) in chips.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&format!(
                                                        "{} {} ({}->{})",
                                                        theme::ICON_SETTINGS, name, inputs, outputs
                                                    ))
                                                    .color(theme::TEXT_PRIMARY.egui()),
                                                );

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui
                                                            .small_button(
                                                                egui::RichText::new(theme::ICON_DELETE)
                                                                    .color(theme::ACCENT_ERROR.egui()),
                                                            )
                                                            .on_hover_text("Delete chip")
                                                            .clicked()
                                                        {
                                                            chip_to_delete = Some(ci);
                                                        }

                                                        // Move to folder button
                                                        egui::ComboBox::from_id_salt(format!("move_chip_{}", ci))
                                                            .selected_text("Move to...")
                                                            .width(100.0)
                                                            .show_ui(ui, |ui| {
                                                                if self.ui.global_lib_selected_folder.is_some() {
                                                                    if ui.selectable_label(false, "Ungrouped").clicked() {
                                                                        chip_to_move = Some((ci, None));
                                                                    }
                                                                }
                                                                for (fi, folder) in self.global_library.folders.iter().enumerate() {
                                                                    if self.ui.global_lib_selected_folder != Some(fi) {
                                                                        if ui.selectable_label(false, &folder.name).clicked() {
                                                                            chip_to_move = Some((ci, Some(fi)));
                                                                        }
                                                                    }
                                                                }
                                                            });
                                                    },
                                                );
                                            });
                                            ui.add_space(3.0);
                                        }

                                        // Process delete
                                        if let Some(ci) = chip_to_delete {
                                            match self.ui.global_lib_selected_folder {
                                                Some(fi) if fi < self.global_library.folders.len() => {
                                                    if ci < self.global_library.folders[fi].chips.len() {
                                                        self.global_library.folders[fi].chips.remove(ci);
                                                    }
                                                }
                                                _ => {
                                                    if ci < self.global_library.ungrouped.len() {
                                                        self.global_library.ungrouped.remove(ci);
                                                    }
                                                }
                                            }
                                            self.sync_engine_library();
                                            global_library::save_global_library(&self.global_library);
                                        }

                                        // Process move
                                        if let Some((ci, target_folder)) = chip_to_move {
                                            let chip = match self.ui.global_lib_selected_folder {
                                                Some(fi) if fi < self.global_library.folders.len() => {
                                                    if ci < self.global_library.folders[fi].chips.len() {
                                                        Some(self.global_library.folders[fi].chips.remove(ci))
                                                    } else {
                                                        None
                                                    }
                                                }
                                                _ => {
                                                    if ci < self.global_library.ungrouped.len() {
                                                        Some(self.global_library.ungrouped.remove(ci))
                                                    } else {
                                                        None
                                                    }
                                                }
                                            };

                                            if let Some(chip) = chip {
                                                match target_folder {
                                                    Some(fi) if fi < self.global_library.folders.len() => {
                                                        self.global_library.folders[fi].chips.push(chip);
                                                    }
                                                    _ => {
                                                        self.global_library.ungrouped.push(chip);
                                                    }
                                                }
                                            }
                                            self.sync_engine_library();
                                            global_library::save_global_library(&self.global_library);
                                        }
                                    });
                            });
                        });
                    });
                });

                // Bottom buttons
                ui.vertical_centered(|ui| {
                    let button_y = screen_rect.height() * 0.85;
                    ui.add_space(button_y - ui.cursor().top());

                    ui.horizontal(|ui| {
                        ui.add_space(screen_rect.width() * 0.25);

                        #[cfg(not(target_os = "android"))]
                        if ui
                            .add_sized(
                                [200.0, 45.0],
                                egui::Button::new(
                                    egui::RichText::new(format!("{} Import from Project", theme::ICON_FOLDER))
                                        .size(18.0)
                                        .color(theme::TEXT_PRIMARY.egui()),
                                )
                                .fill(theme::BG_CANVAS.egui()),
                            )
                            .clicked()
                        {
                            self.import_project_to_global_library();
                        }

                        ui.add_space(20.0);

                        if ui
                            .add_sized(
                                [150.0, 45.0],
                                egui::Button::new(
                                    egui::RichText::new("Back")
                                        .size(20.0)
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
            });
    }

    /// Import chips from a project file into the global library.
    #[cfg(not(target_os = "android"))]
    fn import_project_to_global_library(&mut self) {
        use crate::editor::persistence::ProjectFile;

        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Logic Simulator Projects", &["logic", "json"])
            .set_directory(".")
            .pick_file()
        {
            const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;
            if let Ok(file) = std::fs::File::open(&path) {
                let mut contents = String::new();
                use std::io::Read;
                if file
                    .take(MAX_FILE_SIZE)
                    .read_to_string(&mut contents)
                    .is_ok()
                {
                    if let Ok(project) = serde_json::from_str::<ProjectFile>(&contents) {
                        self.global_library.import_from_project(&project.library);
                        self.sync_engine_library();
                        global_library::save_global_library(&self.global_library);
                    }
                }
            }
        }
    }

    /// Sync the engine library from the global library (rebuilds the flat list).
    pub(crate) fn sync_engine_library(&mut self) {
        self.engine.library = self.global_library.to_flat_list();
    }
}
