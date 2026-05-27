use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USTNode {
    pub id: usize,
    pub tag: String,
    pub title: String,
    pub alt_text: Option<String>,
    pub children: Vec<USTNode>,
}

#[derive(Serialize, Deserialize)]
pub struct USTRegistry {
    pub root: Option<USTNode>,
    pub selected_node_id: Option<usize>,
    pub next_node_id: usize,
    pub audit_findings: Vec<(String, String, String)>, // (checkpoint, severity, message)
}

impl Default for USTRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl USTRegistry {
    pub fn new() -> Self {
        Self {
            root: None,
            selected_node_id: None,
            next_node_id: 1,
            audit_findings: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.selected_node_id = None;
        self.next_node_id = 1;
        self.audit_findings.clear();
    }

    /// Generates a realistic mock UST logical tree from page layouts.
    pub fn initialize_mock_tree(&mut self, total_pages: usize) {
        let mut doc_node = USTNode {
            id: 0,
            tag: "Document".to_string(),
            title: "PDF Document Catalog".to_string(),
            alt_text: None,
            children: Vec::new(),
        };

        let mut next_id = 1;

        for i in 0..total_pages {
            let page_node = USTNode {
                id: next_id,
                tag: "Part".to_string(),
                title: format!("Page {} Section", i + 1),
                alt_text: None,
                children: vec![
                    USTNode {
                        id: next_id + 1,
                        tag: "H1".to_string(),
                        title: format!("Heading of Page {}", i + 1),
                        alt_text: None,
                        children: Vec::new(),
                    },
                    USTNode {
                        id: next_id + 2,
                        tag: "P".to_string(),
                        title: format!("Paragraph content for page {}", i + 1),
                        alt_text: None,
                        children: Vec::new(),
                    },
                    USTNode {
                        id: next_id + 3,
                        tag: "Figure".to_string(),
                        title: format!("Illustration on page {}", i + 1),
                        alt_text: None,
                        children: Vec::new(),
                    },
                ],
            };
            doc_node.children.push(page_node);
            next_id += 4;
        }

        self.root = Some(doc_node);
        self.next_node_id = next_id;
    }
}

pub struct SidebarPanel {
    pub active_tab: usize, // 0 = Tags, 1 = Matterhorn Audit
    pub alt_text_edit_buffer: String,
}

impl Default for SidebarPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarPanel {
    pub fn new() -> Self {
        Self {
            active_tab: 0,
            alt_text_edit_buffer: String::new(),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        registry: &mut USTRegistry,
    ) {
        ui.vertical(|ui| {
            // Tab Selection Headers
            ui.horizontal(|ui| {
                if ui.selectable_label(self.active_tab == 0, "🏷️ Tags Tree").clicked() {
                    self.active_tab = 0;
                }
                ui.separator();
                let audit_count = registry.audit_findings.len();
                let audit_title = if audit_count > 0 {
                    format!("🚨 Matterhorn ({})", audit_count)
                } else {
                    "✅ Matterhorn".to_string()
                };
                if ui.selectable_label(self.active_tab == 1, audit_title).clicked() {
                    self.active_tab = 1;
                }
            });

            ui.separator();

            // Active Tab Content
            if self.active_tab == 0 {
                self.show_tags_tab(ui, registry);
            } else {
                self.show_audit_tab(ui, registry);
            }
        });
    }

    fn show_tags_tab(&mut self, ui: &mut egui::Ui, registry: &mut USTRegistry) {
        ui.label("Collapse/expand structural tags to explore accessibility relationships:");
        ui.add_space(5.0);

        let mut selected_node_id = registry.selected_node_id;

        egui::ScrollArea::vertical().id_salt("tag_tree_scroll").show(ui, |ui| {
            if let Some(ref mut root) = registry.root {
                Self::render_node_recursive(ui, root, &mut selected_node_id, &mut self.alt_text_edit_buffer);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No structure tree loaded. Please load a valid PDF.");
                });
            }
        });

        registry.selected_node_id = selected_node_id;
    }

    fn render_node_recursive(
        ui: &mut egui::Ui,
        node: &mut USTNode,
        selected_node_id: &mut Option<usize>,
        alt_edit_buf: &mut String,
    ) {
        let is_selected = *selected_node_id == Some(node.id);
        let header_label = format!("<{}>  {}", node.tag, node.title);

        ui.horizontal(|ui| {
            // Collapsing tree header
            let _response = ui.collapsing(&header_label, |ui| {
                let mut move_up = None;
                let mut move_down = None;

                let children_len = node.children.len();
                for idx in 0..children_len {
                    ui.horizontal(|ui| {
                        if ui.small_button("▲").clicked() {
                            move_up = Some(idx);
                        }
                        if ui.small_button("▼").clicked() {
                            move_down = Some(idx);
                        }
                        ui.add_space(5.0);

                        // Direct in-place mutation recursion
                        let child = &mut node.children[idx];
                        ui.indent(child.id, |ui| {
                            Self::render_node_recursive(ui, child, selected_node_id, alt_edit_buf);
                        });
                    });
                }

                // Handle reordering mutably on the child vector
                if let Some(idx) = move_up {
                    if idx > 0 {
                        node.children.swap(idx, idx - 1);
                    }
                }
                if let Some(idx) = move_down {
                    if idx + 1 < node.children.len() {
                        node.children.swap(idx, idx + 1);
                    }
                }
            });

            // Alt-Text Studio drawer trigger
            if ui.button("✏️ Alt").clicked() {
                *selected_node_id = Some(node.id);
                *alt_edit_buf = node.alt_text.clone().unwrap_or_default();
            }

            // Quick tag cycle
            if ui.button("🔁 Tag").clicked() {
                node.tag = match node.tag.as_str() {
                    "H1" => "H2".to_string(),
                    "H2" => "P".to_string(),
                    "P" => "H1".to_string(),
                    _ => "P".to_string(),
                };
            }
        });

        // Alt-Text Studio drawer editing logic
        if is_selected {
            ui.indent(node.id, |ui| {
                ui.group(|ui| {
                    ui.label("📝 Alt-Text Studio (Remediation)");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(alt_edit_buf);
                        if ui.button("Save").clicked() {
                            node.alt_text = if alt_edit_buf.trim().is_empty() {
                                None
                            } else {
                                Some(alt_edit_buf.clone())
                            };
                            *selected_node_id = None; // close drawer
                        }
                    });
                });
            });
        }
    }

    fn show_audit_tab(&mut self, ui: &mut egui::Ui, registry: &mut USTRegistry) {
        ui.label("Matterhorn Protocol / UA-2 Accessibility Checklist:");
        ui.add_space(5.0);

        egui::ScrollArea::vertical().id_salt("audit_scroll").show(ui, |ui| {
            if registry.audit_findings.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::GREEN, "🎉 100% Matterhorn Compliance! No errors found.");
                });
            } else {
                for (checkpoint, severity, message) in &registry.audit_findings {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, "🚨");
                            ui.colored_label(egui::Color32::LIGHT_RED, checkpoint);
                            ui.label(format!("({})", severity));
                        });
                        ui.label(message);
                    });
                    ui.add_space(5.0);
                }
            }
        });
    }
}
