use arboard::Clipboard;
use eframe::egui;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
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
    matcher: SkimMatcherV2,
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
            matcher: SkimMatcherV2::default(),
        };

        if let Some(first) = app.snippets.first().cloned() {
            app.selected_id = Some(first.id);
            app.load_into_editor(&first);
        }

        app
    }
}

impl SnippetApp {
    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.search.trim();

        if q.is_empty() {
            return (0..self.snippets.len()).collect();
        }

        let mut scored: Vec<(usize, i64)> = self
            .snippets
            .iter()
            .enumerate()
            .filter_map(|(i, snippet)| {
                let tags = snippet.tags.join(" ");
                let haystack = format!(
                    "{} {} {} {}",
                    snippet.title, snippet.language, tags, snippet.code
                );

                self.matcher.fuzzy_match(&haystack, q).map(|score| (i, score))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(i, _)| i).collect()
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

    fn copy_code(&mut self) {
        if self.code_input.trim().is_empty() {
            self.status = "No code to copy".to_string();
            return;
        }

        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(self.code_input.clone()) {
                Ok(_) => self.status = "Code copied to clipboard".to_string(),
                Err(e) => self.status = format!("Clipboard error: {}", e),
            },
            Err(e) => {
                self.status = format!("Clipboard unavailable: {}", e);
            }
        }
    }

    fn save_all(&mut self) {
        match save_snippets(&self.snippets) {
            Ok(_) => {}
            Err(e) => {
                self.status = format!("Save error: {}", e);
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut new_pressed = false;
        let mut save_pressed = false;
        let mut delete_pressed = false;
        let mut copy_pressed = false;

        ctx.input(|i| {
            new_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::N);
            save_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::S);
            delete_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::D);
            copy_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::C);
        });

        if new_pressed {
            self.create_new_snippet();
        }

        if save_pressed {
            self.save_current();
        }

        if delete_pressed {
            self.delete_current();
        }

        if copy_pressed {
            self.copy_code();
        }
    }

    fn select_snippet(&mut self, id: Uuid) {
        self.selected_id = Some(id);

        if let Some(snippet) = self.snippets.iter().find(|s| s.id == id).cloned() {
            self.load_into_editor(&snippet);
            self.status = "Snippet selected".to_string();
        }
    }

    fn format_tags(tags: &[String]) -> String {
        if tags.is_empty() {
            "no tags".to_string()
        } else {
            tags.join(", ")
        }
    }

    fn selected_snippet_label(&self) -> String {
        match self.selected_id {
            Some(id) => {
                if let Some(snippet) = self.snippets.iter().find(|s| s.id == id) {
                    if snippet.title.trim().is_empty() {
                        "Untitled".to_string()
                    } else {
                        snippet.title.clone()
                    }
                } else {
                    "Unknown".to_string()
                }
            }
            None => "None".to_string(),
        }
    }
}

impl eframe::App for SnippetApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.heading("Snippet Manager v0.0.3");
                ui.separator();

                ui.label("Search");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text("fuzzy search: qs, rgx, srv"),
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

                if ui.button("Copy Code").clicked() {
                    self.copy_code();
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Shortcuts:");
                ui.monospace("Ctrl+N");
                ui.label("New");
                ui.monospace("Ctrl+S");
                ui.label("Save");
                ui.monospace("Ctrl+D");
                ui.label("Delete");
                ui.monospace("Ctrl+C");
                ui.label("Copy code");
            });

            ui.add_space(2.0);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Status: {}", self.status));
                ui.separator();
                ui.label(format!("Total snippets: {}", self.snippets.len()));
                ui.separator();
                ui.label(format!("Selected: {}", self.selected_snippet_label()));
            });
        });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Snippets");
                ui.separator();

                let filtered = self.filtered_indices();

                if filtered.is_empty() {
                    ui.label("No snippets found");
                    return;
                }

                let mut clicked_id: Option<Uuid> = None;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for index in filtered {
                        let snippet = &self.snippets[index];
                        let selected = self.selected_id == Some(snippet.id);

                        let title = if snippet.title.trim().is_empty() {
                            "Untitled"
                        } else {
                            &snippet.title
                        };

                        let meta = format!(
                            "{}  |  {}",
                            if snippet.language.trim().is_empty() {
                                "unknown"
                            } else {
                                &snippet.language
                            },
                            Self::format_tags(&snippet.tags)
                        );

                        let code_preview = snippet
                            .code
                            .lines()
                            .find(|line| !line.trim().is_empty())
                            .unwrap_or("")
                            .trim();

                        let code_preview = if code_preview.len() > 45 {
                            format!("{}...", &code_preview[..45])
                        } else {
                            code_preview.to_string()
                        };

                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.set_width(ui.available_width());

                            if ui
                                .selectable_label(selected, egui::RichText::new(title).strong())
                                .clicked()
                            {
                                clicked_id = Some(snippet.id);
                            }

                            ui.label(
                                egui::RichText::new(meta)
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );

                            if !code_preview.is_empty() {
                                ui.label(
                                    egui::RichText::new(code_preview)
                                        .small()
                                        .italics()
                                        .color(ui.visuals().weak_text_color()),
                                );
                            }
                        });

                        ui.add_space(6.0);
                    }
                });

                if let Some(id) = clicked_id {
                    self.select_snippet(id);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Editor");
            ui.separator();

            ui.label("Title");
            ui.add(egui::TextEdit::singleline(&mut self.title_input));

            ui.add_space(6.0);

            ui.label("Language");
            ui.add(egui::TextEdit::singleline(&mut self.language_input));

            ui.add_space(6.0);

            ui.label("Tags");
            ui.add(
                egui::TextEdit::singleline(&mut self.tags_input)
                    .hint_text("rust, algorithm, async"),
            );

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Code");
                if ui.button("Copy Current Code").clicked() {
                    self.copy_code();
                }
            });

            ui.add(
                egui::TextEdit::multiline(&mut self.code_input)
                    .desired_rows(28)
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
        Ok(content) => content,
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
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 760.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Snippet Manager v0.0.3",
        options,
        Box::new(|_cc| Ok(Box::new(SnippetApp::default()))),
    )
}