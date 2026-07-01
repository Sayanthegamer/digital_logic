use crate::engine::ComponentType;
use crate::editor::theme;

use super::Editor;

impl Editor {
    // Helper to draw the categorized vertical catalog (IDE Toolbox style)
    pub(crate) fn draw_catalog_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("{} Toolbox", theme::ICON_FOLDER));
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut self.ui.catalog_search_text);
        });
        ui.add_space(5.0);
        let search = self.ui.catalog_search_text.to_lowercase();

        egui::CollapsingHeader::new("Primitives")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if (search.is_empty() || "nand gate".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand)),
                            format!("{} NAND Gate", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand));
                        }
                    
                    if (search.is_empty() || "clock".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Clock)),
                            format!("{} Clock", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Clock));
                        }
                });
            });

        ui.add_space(5.0);

        egui::CollapsingHeader::new("Bus & Routing")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if (search.is_empty() || "bus junction".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Junction)),
                            format!("{} Bus Junction", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Junction));
                        }
                    if (search.is_empty() || "tri-state buffer".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::TriStateBuffer)),
                            format!("{} Tri-State Buffer", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::TriStateBuffer));
                        }
                });
            });

        ui.add_space(5.0);

        egui::CollapsingHeader::new("Input / Output")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if (search.is_empty() || "switch input".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input)),
                            format!("{} Switch Input", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input));
                        }
                    if (search.is_empty() || "light output".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output)),
                            format!("{} Light Output", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output));
                        }
                    if (search.is_empty() || "7-segment".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::SevenSegment)),
                            format!("{} 7-Segment", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::SevenSegment));
                        }
                });
            });

        ui.add_space(5.0);

        egui::CollapsingHeader::new("Custom Chips")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    let total_chips = self.engine.library.len();
                    if total_chips > 0 {
                        let mut move_operation = None;
                        for (idx, bp) in self.engine.library.iter().enumerate() {
                            let is_sel = self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            let mut response = ui.selectable_label(is_sel, format!("{} {}", theme::ICON_FOLDER, bp.name));
                            
                            // Add drag sensing to the label
                            response = response.interact(egui::Sense::drag());
                            
                            if response.drag_started() {
                                self.ui.dragging_catalog_idx = Some(idx);
                            }
                            
                            if let Some(dragged_idx) = self.ui.dragging_catalog_idx {
                                if response.hovered() {
                                    self.ui.drag_hovered_idx = Some(idx);
                                }
                                
                                if self.ui.drag_hovered_idx == Some(idx) && dragged_idx != idx {
                                    let rect = response.rect;
                                    let y = if idx > dragged_idx { rect.max.y } else { rect.min.y };
                                    ui.painter().hline(rect.min.x..=rect.max.x, y, (2.0, egui::Color32::WHITE));
                                }
                            }
                            
                            if response.clicked() {
                                self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            }
                        }
                        
                        if ui.input(|i| i.pointer.any_released()) {
                            if let (Some(from), Some(to)) = (self.ui.dragging_catalog_idx, self.ui.drag_hovered_idx) {
                                if from != to {
                                    move_operation = Some((from, to));
                                }
                            }
                            self.ui.dragging_catalog_idx = None;
                            self.ui.drag_hovered_idx = None;
                        }
                        
                        if let Some((from, to)) = move_operation {
                            self.remap_library_chip(from, to);
                        }
                    } else {
                        ui.weak("No custom chips created.");
                    }
                });
            });

        ui.add_space(5.0);
        
        egui::CollapsingHeader::new("Annotations")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if (search.is_empty() || "text note".contains(&search) || "annotation".contains(&search))
                        && ui.selectable_label(
                            self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceAnnotation),
                            format!("{} Text Note", theme::ICON_SETTINGS),
                        ).clicked() {
                            self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceAnnotation);
                        }
                });
            });


    }
}
