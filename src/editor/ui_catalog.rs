use crate::engine::ComponentType;
use crate::editor::theme;

use super::Editor;

impl Editor {
    // Helper to draw the categorized vertical catalog (IDE Toolbox style)
    pub(crate) fn draw_catalog_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("{} Toolbox", theme::ICON_FOLDER));
        ui.add_space(5.0);

        egui::CollapsingHeader::new("Primitives")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if ui.selectable_label(
                        self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand)),
                        format!("{} NAND Gate", theme::ICON_SETTINGS),
                    ).clicked() {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand));
                    }
                    
                    if ui.selectable_label(
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
                    if ui.selectable_label(
                        self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Junction)),
                        format!("{} Bus Junction", theme::ICON_SETTINGS),
                    ).clicked() {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Junction));
                    }
                    if ui.selectable_label(
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
                    if ui.selectable_label(
                        self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input)),
                        format!("{} Switch Input", theme::ICON_SETTINGS),
                    ).clicked() {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input));
                    }
                    if ui.selectable_label(
                        self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output)),
                        format!("{} Light Output", theme::ICON_SETTINGS),
                    ).clicked() {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output));
                    }
                    if ui.selectable_label(
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
                        for (idx, bp) in self.engine.library.iter().enumerate() {
                            let is_sel = self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            if ui.selectable_label(is_sel, format!("{} {}", theme::ICON_FOLDER, bp.name)).clicked() {
                                self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            }
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
                    if ui.selectable_label(
                        self.canvas.selected_tool == Some(super::types::ActiveTool::PlaceAnnotation),
                        format!("{} Text Note", theme::ICON_SETTINGS),
                    ).clicked() {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceAnnotation);
                    }
                });
            });

        ui.add_space(10.0);
        ui.separator();
        if ui.button("❌ Clear Tool").clicked() {
            self.canvas.selected_tool = None;
        }
    }
}
