use crate::engine::ComponentType;

use super::Editor;

impl Editor {
    // Helper to draw the scrollable catalog with paging indicators/arrows
    pub(crate) fn draw_catalog_ui(&mut self, ui: &mut egui::Ui) {
        let mut scroll_by = self.catalog_scroll_request.take();

        // Handle vertical mouse wheel as horizontal scroll
        let scroll_delta = ui.input(|i| i.raw_scroll_delta);
        if scroll_delta.y != 0.0 {
            scroll_by = Some(scroll_delta.y * 3.0); // Amplify for responsiveness
        }

        ui.horizontal(|ui| {
            // Scroll Left Button
            if ui.button("◀").on_hover_text("Scroll Left").clicked() {
                self.catalog_scroll_request = Some(150.0);
            }

            egui::ScrollArea::horizontal()
                .show(ui, |ui| {
                    if let Some(delta) = scroll_by {
                        ui.scroll_with_delta(egui::vec2(delta, 0.0));
                    }
                    ui.horizontal(|ui| {
                        if ui.selectable_label(
                            self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input)),
                            "📥 Input",
                        ).clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Input));
                        }
                        if ui.selectable_label(
                            self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output)),
                            "📤 Output",
                        ).clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Output));
                        }
                        if ui.selectable_label(
                            self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand)),
                            "⛩ NAND",
                        ).clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Nand));
                        }
                        if ui.selectable_label(
                            self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::Clock)),
                            "⏱ Clock",
                        ).clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::Clock));
                        }
                        if ui.selectable_label(
                            self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::SevenSegment)),
                            "🖩 7-Seg",
                        ).clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::SevenSegment));
                        }
                        if ui.selectable_label(
                            self.selected_tool == Some(super::types::ActiveTool::PlaceAnnotation),
                            "📝 Text",
                        ).clicked() {
                            self.selected_tool = Some(super::types::ActiveTool::PlaceAnnotation);
                        }
                        
                        ui.separator();

                        // --- Custom Chips pagination/scrolling with arrows ---
                        let chips_per_page = 3;
                        let total_chips = self.library.len();
                        
                        if total_chips > 0 {
                            // Previous page arrow button
                            let has_prev = self.catalog_page > 0;
                            if ui.add_enabled(has_prev, egui::Button::new("◀")).clicked() {
                                self.catalog_page = self.catalog_page.saturating_sub(1);
                            }

                            // Page window bounds
                            let start_idx = self.catalog_page * chips_per_page;
                            let end_idx = (start_idx + chips_per_page).min(total_chips);

                            // Reset page bounds if library changes or shrinks
                            if start_idx >= total_chips && self.catalog_page > 0 {
                                self.catalog_page = 0;
                            }

                            for idx in start_idx..end_idx {
                                if let Some(bp) = self.library.get(idx) {
                                    let is_sel = self.selected_tool == Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                                    if ui.selectable_label(is_sel, format!("📦 {}", bp.name)).clicked() {
                                        self.selected_tool = Some(super::types::ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                                    }
                                }
                            }

                            // Next page arrow button
                            let has_next = end_idx < total_chips;
                            if ui.add_enabled(has_next, egui::Button::new("▶")).clicked() {
                                self.catalog_page += 1;
                            }
                        } else {
                            ui.weak("No custom chips");
                        }

                        ui.separator();
                        if ui.button("❌ Clear").clicked() {
                            self.selected_tool = None;
                        }
                    });
                });

            // Scroll Right Button
            if ui.button("▶").on_hover_text("Scroll Right").clicked() {
                self.catalog_scroll_request = Some(-150.0);
            }
        });
    }
}
