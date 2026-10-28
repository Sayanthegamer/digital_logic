#![allow(clippy::collapsible_if)]
use super::Editor;
use super::state::AppMode;
use crate::editor::global_library;
use crate::editor::theme;

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
                                    self.global_library
                                        .folders
                                        .push(global_library::ChipFolder {
                                            name: format!(
                                                "Folder {}",
                                                self.global_library.folders.len() + 1
                                            ),
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
                                        let is_ungrouped_selected =
                                            self.ui.global_lib_selected_folder.is_none();
                                        if ui
                                            .selectable_label(
                                                is_ungrouped_selected,
                                                format!(
                                                    "{} Ungrouped ({})",
                                                    theme::ICON_FOLDER,
                                                    self.global_library.ungrouped.len()
                                                ),
                                            )
                                            .clicked()
                                        {
                                            self.ui.global_lib_selected_folder = None;
                                        }

                                        ui.add_space(3.0);

                                        let mut folder_to_delete = None;
                                        for (fi, folder) in
                                            self.global_library.folders.iter().enumerate()
                                        {
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

                                                let selected =
                                                    self.ui.global_lib_selected_folder == Some(fi);
                                                if ui
                                                    .selectable_label(
                                                        selected,
                                                        format!(
                                                            "{} ({} chips)",
                                                            folder.name,
                                                            folder.chips.len()
                                                        ),
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
                                                    .on_hover_text(
                                                        "Delete folder (chips move to Ungrouped)",
                                                    )
                                                    .clicked()
                                                {
                                                    folder_to_delete = Some(fi);
                                                }
                                            });
                                        }

                                        if let Some(fi) = folder_to_delete {
                                            // Compute exact flat index remapping before moving folder chips to ungrouped
                                            let old_total =
                                                self.global_library.to_flat_list().len();
                                            let mut tagged_ungrouped: Vec<usize> =
                                                (0..self.global_library.ungrouped.len()).collect();
                                            let mut offset = self.global_library.ungrouped.len();
                                            let mut tagged_folders: Vec<Vec<usize>> = Vec::new();
                                            for folder in &self.global_library.folders {
                                                let len = folder.chips.len();
                                                tagged_folders
                                                    .push((offset..offset + len).collect());
                                                offset += len;
                                            }
                                            let moved_indices = tagged_folders.remove(fi);
                                            tagged_ungrouped.extend(moved_indices);
                                            let mut new_flat_order = tagged_ungrouped;
                                            for tf in tagged_folders {
                                                new_flat_order.extend(tf);
                                            }
                                            let mut map = vec![None; old_total];
                                            for (new_idx, &old_idx) in
                                                new_flat_order.iter().enumerate()
                                            {
                                                map[old_idx] = Some(new_idx);
                                            }

                                            // Move chips to ungrouped before deleting
                                            let chips =
                                                self.global_library.folders[fi].chips.clone();
                                            self.global_library.ungrouped.extend(chips);
                                            self.global_library.folders.remove(fi);
                                            if self.ui.global_lib_selected_folder == Some(fi) {
                                                self.ui.global_lib_selected_folder = None;
                                            }
                                            self.sync_engine_library();
                                            self.apply_library_index_map(&map);
                                            global_library::save_global_library(
                                                &self.global_library,
                                            );
                                            self.compile();
                                        }
                                    });
                            });

                            ui.separator();

                            // Right panel: Chip list for selected folder
                            let right_w = (screen_rect.width() - margin * 2.0) * 0.60;
                            ui.vertical(|ui| {
                                ui.set_width(right_w);
                                ui.set_max_height(panel_height);

                                let (folder_name, chips_ref_len) =
                                    match self.ui.global_lib_selected_folder {
                                        Some(fi) if fi < self.global_library.folders.len() => (
                                            self.global_library.folders[fi].name.clone(),
                                            self.global_library.folders[fi].chips.len(),
                                        ),
                                        _ => (
                                            "Ungrouped".to_string(),
                                            self.global_library.ungrouped.len(),
                                        ),
                                    };

                                ui.heading(
                                    egui::RichText::new(format!(
                                        "{} — {} chips",
                                        folder_name, chips_ref_len
                                    ))
                                    .size(20.0)
                                    .color(theme::TEXT_PRIMARY.egui()),
                                );
                                ui.add_space(5.0);

                                // Folder rename (if a folder is selected)
                                if let Some(fi) = self.ui.global_lib_selected_folder {
                                    if fi < self.global_library.folders.len() {
                                        ui.horizontal(|ui| {
                                            ui.label("Rename:");
                                            let mut name =
                                                self.global_library.folders[fi].name.clone();
                                            if ui.text_edit_singleline(&mut name).changed() {
                                                self.global_library.folders[fi].name = name;
                                                global_library::save_global_library(
                                                    &self.global_library,
                                                );
                                            }
                                        });

                                        // Folder colour
                                        ui.horizontal(|ui| {
                                            ui.label("Folder Color:");
                                            let mut col = self.global_library.folders[fi]
                                                .color
                                                .unwrap_or([0.4, 0.45, 0.85, 1.0]);
                                            if ui
                                                .color_edit_button_rgba_unmultiplied(&mut col)
                                                .changed()
                                            {
                                                self.global_library.folders[fi].color = Some(col);
                                                global_library::save_global_library(
                                                    &self.global_library,
                                                );
                                            }
                                            if ui.small_button("Reset").clicked() {
                                                self.global_library.folders[fi].color = None;
                                                global_library::save_global_library(
                                                    &self.global_library,
                                                );
                                            }
                                        });

                                        ui.add_space(5.0);
                                    }
                                }

                                egui::ScrollArea::vertical()
                                    .id_salt("global_lib_chips")
                                    .max_height(panel_height - 120.0)
                                    .show(ui, |ui| {
                                        let chips: Vec<(String, usize, usize)> = match self
                                            .ui
                                            .global_lib_selected_folder
                                        {
                                            Some(fi) if fi < self.global_library.folders.len() => {
                                                self.global_library.folders[fi]
                                                    .chips
                                                    .iter()
                                                    .map(|bp| {
                                                        (bp.name.clone(), bp.inputs, bp.outputs)
                                                    })
                                                    .collect()
                                            }
                                            _ => self
                                                .global_library
                                                .ungrouped
                                                .iter()
                                                .map(|bp| (bp.name.clone(), bp.inputs, bp.outputs))
                                                .collect(),
                                        };

                                        if chips.is_empty() {
                                            ui.label(
                                                egui::RichText::new("No chips in this group")
                                                    .color(theme::TEXT_SECONDARY.egui()),
                                            );
                                        }

                                        let mut chip_to_delete = None;
                                        let mut chip_to_move = None;

                                        for (ci, (name, inputs, outputs)) in
                                            chips.iter().enumerate()
                                        {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} {} ({}->{})",
                                                        theme::ICON_SETTINGS,
                                                        name,
                                                        inputs,
                                                        outputs
                                                    ))
                                                    .color(theme::TEXT_PRIMARY.egui()),
                                                );

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if ui
                                                            .small_button(
                                                                egui::RichText::new(
                                                                    theme::ICON_DELETE,
                                                                )
                                                                .color(theme::ACCENT_ERROR.egui()),
                                                            )
                                                            .on_hover_text("Delete chip")
                                                            .clicked()
                                                        {
                                                            chip_to_delete = Some(ci);
                                                        }

                                                        // Move to folder button
                                                        egui::ComboBox::from_id_salt(format!(
                                                            "move_chip_{}",
                                                            ci
                                                        ))
                                                        .selected_text("Move to...")
                                                        .width(100.0)
                                                        .show_ui(ui, |ui| {
                                                            if self
                                                                .ui
                                                                .global_lib_selected_folder
                                                                .is_some()
                                                            {
                                                                if ui
                                                                    .selectable_label(
                                                                        false,
                                                                        "Ungrouped",
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    chip_to_move = Some((ci, None));
                                                                }
                                                            }
                                                            for (fi, folder) in self
                                                                .global_library
                                                                .folders
                                                                .iter()
                                                                .enumerate()
                                                            {
                                                                if self
                                                                    .ui
                                                                    .global_lib_selected_folder
                                                                    != Some(fi)
                                                                {
                                                                    if ui
                                                                        .selectable_label(
                                                                            false,
                                                                            &folder.name,
                                                                        )
                                                                        .clicked()
                                                                    {
                                                                        chip_to_move =
                                                                            Some((ci, Some(fi)));
                                                                    }
                                                                }
                                                            }
                                                        });
                                                    },
                                                );
                                            });
                                            ui.add_space(3.0);
                                        }

                                        // Process delete using centralized flat-index remapper
                                        if let Some(ci) = chip_to_delete {
                                            let flat_idx_opt = match self
                                                .ui
                                                .global_lib_selected_folder
                                            {
                                                Some(fi)
                                                    if fi < self.global_library.folders.len() =>
                                                {
                                                    if ci
                                                        < self.global_library.folders[fi]
                                                            .chips
                                                            .len()
                                                    {
                                                        let mut offset =
                                                            self.global_library.ungrouped.len();
                                                        for f_idx in 0..fi {
                                                            offset += self.global_library.folders
                                                                [f_idx]
                                                                .chips
                                                                .len();
                                                        }
                                                        Some(offset + ci)
                                                    } else {
                                                        None
                                                    }
                                                }
                                                _ => {
                                                    if ci < self.global_library.ungrouped.len() {
                                                        Some(ci)
                                                    } else {
                                                        None
                                                    }
                                                }
                                            };
                                            if let Some(flat_idx) = flat_idx_opt {
                                                self.delete_library_chip(flat_idx);
                                            }
                                        }

                                        // Process move with full flat-index remapping
                                        if let Some((ci, target_folder)) = chip_to_move {
                                            let from_loc = match self.ui.global_lib_selected_folder
                                            {
                                                Some(fi)
                                                    if fi < self.global_library.folders.len() =>
                                                {
                                                    Some(global_library::ChipLocation::InFolder(
                                                        fi, ci,
                                                    ))
                                                }
                                                _ => Some(global_library::ChipLocation::Ungrouped(
                                                    ci,
                                                )),
                                            };
                                            if let Some(from) = from_loc {
                                                self.move_global_library_chip(from, target_folder);
                                            }
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
                                    egui::RichText::new(format!(
                                        "{} Import from Project",
                                        theme::ICON_FOLDER
                                    ))
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

    /// Move a chip in the global library and remap all SubChip flat indices across the editor and blueprints.
    pub fn move_global_library_chip(
        &mut self,
        from: global_library::ChipLocation,
        to_folder: Option<usize>,
    ) {
        let old_total = self.global_library.to_flat_list().len();
        let mut tagged_ungrouped: Vec<usize> = (0..self.global_library.ungrouped.len()).collect();
        let mut offset = self.global_library.ungrouped.len();
        let mut tagged_folders: Vec<Vec<usize>> = Vec::new();
        for folder in &self.global_library.folders {
            let len = folder.chips.len();
            tagged_folders.push((offset..offset + len).collect());
            offset += len;
        }

        let moved_old_idx = match from {
            global_library::ChipLocation::Ungrouped(idx) => {
                if idx < tagged_ungrouped.len() {
                    tagged_ungrouped.remove(idx)
                } else {
                    return;
                }
            }
            global_library::ChipLocation::InFolder(fi, ci) => {
                if fi < tagged_folders.len() && ci < tagged_folders[fi].len() {
                    tagged_folders[fi].remove(ci)
                } else {
                    return;
                }
            }
        };

        match to_folder {
            Some(fi) if fi < tagged_folders.len() => {
                tagged_folders[fi].push(moved_old_idx);
            }
            _ => {
                tagged_ungrouped.push(moved_old_idx);
            }
        }

        let mut new_flat_order = tagged_ungrouped;
        for tf in tagged_folders {
            new_flat_order.extend(tf);
        }

        let mut map = vec![None; old_total];
        for (new_idx, &old_idx) in new_flat_order.iter().enumerate() {
            map[old_idx] = Some(new_idx);
        }

        self.global_library.move_chip(from, to_folder);
        self.sync_engine_library();
        self.apply_library_index_map(&map);
        global_library::save_global_library(&self.global_library);
        self.compile();
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
                    if let Ok(project) = serde_json::from_str::<ProjectFile>(&contents)
                        && crate::engine::Simulator::validate_library(&project.library).is_ok()
                    {
                        let old_ungrouped_len = self.global_library.ungrouped.len();
                        let old_total = self.global_library.to_flat_list().len();
                        self.global_library.import_from_project(&project.library);
                        let new_ungrouped_len = self.global_library.ungrouped.len();
                        let added = new_ungrouped_len.saturating_sub(old_ungrouped_len);
                        self.sync_engine_library();
                        if added > 0 && old_total > old_ungrouped_len {
                            let mut map = Vec::with_capacity(old_total);
                            for i in 0..old_total {
                                if i < old_ungrouped_len {
                                    map.push(Some(i));
                                } else {
                                    map.push(Some(i + added));
                                }
                            }
                            self.apply_library_index_map(&map);
                        }
                        global_library::save_global_library(&self.global_library);
                        self.compile();
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
