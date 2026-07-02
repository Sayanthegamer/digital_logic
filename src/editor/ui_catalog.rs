#![allow(clippy::collapsible_if)]
use crate::editor::theme;
use crate::engine::ComponentType;

use super::Editor;

impl Editor {
    // Helper to draw the categorized vertical catalog (IDE Toolbox style)
    pub(crate) fn draw_catalog_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("{} Toolbox", theme::ICON_FOLDER));
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            ui.label(crate::editor::theme::ICON_SEARCH);
            ui.text_edit_singleline(&mut self.ui.catalog_search_text);
        });
        ui.add_space(5.0);
        let search = self.ui.catalog_search_text.to_lowercase();

        egui::CollapsingHeader::new("Primitives")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if (search.is_empty() || "nand gate".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::Nand,
                                    )),
                                format!("{} NAND Gate", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(
                            ComponentType::Nand,
                        ));
                    }

                    if (search.is_empty() || "clock".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::Clock,
                                    )),
                                format!("{} Clock", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(
                            ComponentType::Clock,
                        ));
                    }
                });
            });

        ui.add_space(5.0);

        egui::CollapsingHeader::new("Bus & Routing")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if (search.is_empty() || "bus junction".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::Junction,
                                    )),
                                format!("{} Bus Junction", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(
                            ComponentType::Junction,
                        ));
                    }
                    if (search.is_empty() || "tri-state buffer".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::TriStateBuffer,
                                    )),
                                format!("{} Tri-State Buffer", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(
                            ComponentType::TriStateBuffer,
                        ));
                    }
                });
            });

        ui.add_space(5.0);

        egui::CollapsingHeader::new("Input / Output")
            .default_open(true)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    if (search.is_empty() || "switch input".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::Input,
                                    )),
                                format!("{} Switch Input", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(
                            ComponentType::Input,
                        ));
                    }
                    if (search.is_empty() || "light output".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::Output,
                                    )),
                                format!("{} Light Output", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(
                            ComponentType::Output,
                        ));
                    }
                    if (search.is_empty() || "7-segment".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::SevenSegment,
                                    )),
                                format!("{} 7-Segment", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceComponent(
                            ComponentType::SevenSegment,
                        ));
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
                            let is_sel = self.canvas.selected_tool
                                == Some(super::types::ActiveTool::PlaceComponent(
                                    ComponentType::SubChip(idx),
                                ));
                            let mut response = ui.selectable_label(
                                is_sel,
                                format!("{} {}", theme::ICON_FOLDER, bp.name),
                            );

                            // Add drag sensing to the label
                            response = response.interact(egui::Sense::drag());

                            if response.drag_started() {
                                self.ui.dragging_catalog_idx = Some(idx);
                            }

                            if let Some(dragged_idx) = self.ui.dragging_catalog_idx {
                                // Dim the item currently being dragged
                                if dragged_idx == idx {
                                    ui.painter().rect_filled(
                                        response.rect,
                                        2.0,
                                        egui::Color32::from_black_alpha(150),
                                    );
                                }

                                // Check if pointer is over this rect, even during a drag
                                if ui.rect_contains_pointer(response.rect) {
                                    self.ui.drag_hovered_idx = Some(idx);
                                }

                                if self.ui.drag_hovered_idx == Some(idx) && dragged_idx != idx {
                                    let rect = response.rect;
                                    let y = if idx > dragged_idx {
                                        rect.max.y
                                    } else {
                                        rect.min.y
                                    };

                                    // Draw a ghost gap indicator
                                    let gap_rect = egui::Rect::from_min_size(
                                        egui::pos2(rect.min.x, y - rect.height() / 2.0),
                                        egui::vec2(rect.width(), rect.height()),
                                    );
                                    ui.painter().rect_filled(
                                        gap_rect,
                                        2.0,
                                        egui::Color32::from_white_alpha(30),
                                    );
                                    ui.painter().hline(
                                        rect.min.x..=rect.max.x,
                                        y,
                                        (2.0, egui::Color32::WHITE),
                                    );
                                }
                            }

                            if response.clicked() {
                                self.canvas.selected_tool =
                                    Some(super::types::ActiveTool::PlaceComponent(
                                        ComponentType::SubChip(idx),
                                    ));
                            }
                        }

                        if let Some(dragged_idx) = self.ui.dragging_catalog_idx {
                            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                                if dragged_idx < self.engine.library.len() {
                                    let bp = &self.engine.library[dragged_idx];
                                    #[allow(clippy::needless_pass_by_value)]
                                    let tooltip_layer = egui::LayerId::new(
                                        egui::Order::Tooltip,
                                        egui::Id::new("dnd_ghost"),
                                    );
                                    let painter = ui.ctx().layer_painter(tooltip_layer);

                                    let rect = egui::Rect::from_min_size(
                                        pointer_pos + egui::vec2(12.0, 12.0),
                                        egui::vec2(180.0, 24.0),
                                    );

                                    painter.rect(
                                        rect,
                                        4.0,
                                        ui.visuals().window_fill().linear_multiply(0.9),
                                        ui.visuals().window_stroke(),
                                        egui::StrokeKind::Middle,
                                    );

                                    let text = format!("{} {}", theme::ICON_FOLDER, bp.name);
                                    painter.text(
                                        rect.min + egui::vec2(8.0, 4.0),
                                        egui::Align2::LEFT_TOP,
                                        text,
                                        egui::FontId::proportional(14.0),
                                        ui.visuals().text_color(),
                                    );
                                }
                            }
                        }

                        if ui.input(|i| i.pointer.any_released()) {
                            if let (Some(from), Some(to)) =
                                (self.ui.dragging_catalog_idx, self.ui.drag_hovered_idx)
                            {
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
                    if (search.is_empty()
                        || "text note".contains(&search)
                        || "annotation".contains(&search))
                        && ui
                            .selectable_label(
                                self.canvas.selected_tool
                                    == Some(super::types::ActiveTool::PlaceAnnotation),
                                format!("{} Text Note", theme::ICON_SETTINGS),
                            )
                            .clicked()
                    {
                        self.canvas.selected_tool = Some(super::types::ActiveTool::PlaceAnnotation);
                    }
                });
            });
    }
}
