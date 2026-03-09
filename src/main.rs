use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

const FILE_PATH: &str = "snippets.json";

#[derive(Serialize, Deserialize, Clone)]
struct Snippet {
    id: Uuid,
    title: String,
    language: String,
    tags: Vec<String>,
    code: String,
}

impl Snippet {
    fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            title: "New Snippet".to_string(),
            language: "rust".to_string(),
            tags: Vec::new(),
            code: String::new(),
        }
    }
}

struct SnippetApp {
    snippets: Vec<Snippet>,
    selected_id: Option<Uuid>,
    search: String,
    status: String,
    title_input: String,
    language_input: String,
    tags_input: String,
    code_input: String,
}

impl Default for SnippetApp {
    fn default() -> Self {
        let snippets = load_snippets();
        let mut app = Self {
            snippets,
            selected_id: None,
            search: String::new(),
            status: "Ready".to_string(),
            title_input: String::new(),
            language_input: String::new(),
            tags_input: String::new(),
            code_input: String::new(),
        };

        if let Some(first) = app.snippets.first().cloned() {
            app.load_into_editor(&first);
            app.selected_id = Some(first.id);
        }

        app
    }
}

impl SnippetApp {
    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.search.trim().to_lowercase();

        if q.is_empty() {
            return (0..self.snippets.len()).collect();
        }

        self.snippets
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.title.to_lowercase().contains(&q)
                    || s.language.to_lowercase().contains(&q)
                    || s.code.to_lowercase().contains(&q)
                    || s.tags.iter().any(|tag| tag.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn load_into_editor(&mut self, snippet: &Snippet) {
        self.title_input = snippet.title.clone();
        self.language_input = snippet.language.clone();
        self.tags_input = snippet.tags.join(", ");
        self.code_input = snippet.code.clone();
    }

    fn clear_editor(&mut self) {
        self.selected_id = None;
        self.title_input.clear();
        self.language_input.clear();
        self.tags_input.clear();
        self.code_input.clear();
    }

    fn create_new_snippet(&mut self) {
        let snippet = Snippet::new();
        self.selected_id = Some(snippet.id);
        self.load_into_editor(&snippet);
        self.snippets.push(snippet);
        self.status = "New snippet created".to_string();
        self.save_all();
    }

    fn save_current(&mut self) {
        let Some(id) = self.selected_id else {
            self.status = "No snippet selected".to_string();
            return;
        };

        if let Some(snippet) = self.snippets.iter_mut().find(|s| s.id == id) {
            snippet.title = self.title_input.trim().to_string();
            snippet.language = self.language_input.trim().to_string();
            snippet.tags = self
                .tags_input
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            snippet.code = self.code_input.clone();

            self.save_all();
            self.status = "Snippet saved".to_string();
        } else {
            self.status = "Snippet not found".to_string();
        }
    }

    fn delete_current(&mut self) {
        let Some(id) = self.selected_id else {
            self.status = "No snippet selected".to_string();
            return;
        };

        let before = self.snippets.len();
        self.snippets.retain(|s| s.id != id);

        if self.snippets.len() == before {
            self.status = "Snippet not found".to_string();
            return;
        }

        if let Some(first) = self.snippets.first().cloned() {
            self.selected_id = Some(first.id);
            self.load_into_editor(&first);
        } else {
            self.clear_editor();
        }

        self.save_all();
        self.status = "Snippet deleted".to_string();
    }

    fn save_all(&mut self) {
        match save_snippets(&self.snippets) {
            Ok(_) => {}
            Err(e) => {
                self.status = format!("Save error: {}", e);
            }
        }
    }
}

impl eframe::App for SnippetApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Snippet Manager v0.0.2");
                ui.separator();
                ui.label("Search:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("title, tag, language, code"),
                );
                if ui.button("New").clicked() {
                    self.create_new_snippet();
                }
                if ui.button("Save").clicked() {
                    self.save_current();
                }
                if ui.button("Delete").clicked() {
                    self.delete_current();
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
                ui.separator();
                ui.label(format!("Total snippets: {}", self.snippets.len()));
            });
        });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Snippets");
                ui.separator();

                let filtered = self.filtered_indices();

                if filtered.is_empty() {
                    ui.label("No snippets found");
                } else {
                    let mut clicked_id: Option<Uuid> = None;

                    for index in filtered {
                        let snippet = &self.snippets[index];
                        let selected = self.selected_id == Some(snippet.id);

                        let title = if snippet.title.trim().is_empty() {
                            "Untitled"
                        } else {
                            &snippet.title
                        };

                        let subtitle = format!(
                            "{} | {}",
                            snippet.language,
                            if snippet.tags.is_empty() {
                                "no tags".to_string()
                            } else {
                                snippet.tags.join(", ")
                            }
                        );

                        ui.vertical(|ui| {
                            if ui.selectable_label(selected, title).clicked() {
                                clicked_id = Some(snippet.id);
                            }
                            ui.label(subtitle);
                        });

                        ui.separator();
                    }

                    if let Some(id) = clicked_id {
                        self.selected_id = Some(id);
                        if let Some(snippet) = self.snippets.iter().find(|s| s.id == id).cloned() {
                            self.load_into_editor(&snippet);
                            self.status = "Snippet selected".to_string();
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Editor");
            ui.separator();

            ui.label("Title");
            ui.add(egui::TextEdit::singleline(&mut self.title_input));

            ui.label("Language");
            ui.add(egui::TextEdit::singleline(&mut self.language_input));

            ui.label("Tags");
            ui.add(
                egui::TextEdit::singleline(&mut self.tags_input)
                    .hint_text("rust, sorting, async"),
            );

            ui.label("Code");
            ui.add(
                egui::TextEdit::multiline(&mut self.code_input)
                    .desired_rows(24)
                    .lock_focus(true)
                    .desired_width(f32::INFINITY),
            );
        });
    }
}

fn load_snippets() -> Vec<Snippet> {
    if !Path::new(FILE_PATH).exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(FILE_PATH) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    if content.trim().is_empty() {
        return Vec::new();
    }

    serde_json::from_str(&content).unwrap_or_default()
}

fn save_snippets(snippets: &[Snippet]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(snippets).map_err(|e| e.to_string())?;
    fs::write(FILE_PATH, json).map_err(|e| e.to_string())
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Snippet Manager v0.0.2",
        options,
        Box::new(|_cc| Ok(Box::new(SnippetApp::default()))),
    )
}