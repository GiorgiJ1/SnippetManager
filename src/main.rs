use arboard::Clipboard;
use eframe::egui;
use eframe::egui::text::{LayoutJob, TextFormat};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
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
    active_tag: Option<String>,
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Default for SnippetApp {
    fn default() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get("base16-ocean.dark")
            .cloned()
            .or_else(|| theme_set.themes.values().next().cloned())
            .unwrap_or_default();

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
            active_tag: None,
            syntax_set,
            theme,
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
        let active_tag = self.active_tag.as_deref();

        if q.is_empty() {
            return self
                .snippets
                .iter()
                .enumerate()
                .filter(|(_, snippet)| match active_tag {
                    Some(tag) => snippet.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)),
                    None => true,
                })
                .map(|(i, _)| i)
                .collect();
        }

        let mut scored: Vec<(usize, i64)> = self
            .snippets
            .iter()
            .enumerate()
            .filter_map(|(i, snippet)| {
                if let Some(tag) = active_tag {
                    if !snippet.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                        return None;
                    }
                }

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

        let new_title = self.title_input.trim().to_string();
        let new_language = self.language_input.trim().to_string();
        let new_tags: Vec<String> = self
            .tags_input
            .split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let new_code = self.code_input.clone();

        if let Some(snippet) = self.snippets.iter_mut().find(|s| s.id == id) {
            snippet.title = new_title;
            snippet.language = new_language;
            snippet.tags = new_tags;
            snippet.code = new_code;

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

    fn import_snippets(&mut self) {
        let Some(path) = FileDialog::new()
            .add_filter("JSON files", &["json"])
            .pick_file()
        else {
            return;
        };

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Vec<Snippet>>(&content) {
                Ok(imported) => {
                    let mut added = 0usize;

                    for snippet in imported {
                        if self.snippets.iter().any(|s| s.id == snippet.id) {
                            continue;
                        }

                        self.snippets.push(snippet);
                        added += 1;
                    }

                    self.save_all();

                    if added == 0 {
                        self.status = "No new snippets were imported".to_string();
                    } else {
                        self.status = format!("Imported {} snippet(s)", added);
                    }

                    if self.selected_id.is_none() {
                        if let Some(first) = self.snippets.first().cloned() {
                            self.selected_id = Some(first.id);
                            self.load_into_editor(&first);
                        }
                    }
                }
                Err(e) => {
                    self.status = format!("Import parse error: {}", e);
                }
            },
            Err(e) => {
                self.status = format!("Import read error: {}", e);
            }
        }
    }

    fn export_snippets(&mut self) {
        let Some(path) = FileDialog::new()
            .set_file_name("snippets_export.json")
            .save_file()
        else {
            return;
        };

        match serde_json::to_string_pretty(&self.snippets) {
            Ok(json) => match fs::write(path, json) {
                Ok(_) => {
                    self.status = "Snippets exported".to_string();
                }
                Err(e) => {
                    self.status = format!("Export write error: {}", e);
                }
            },
            Err(e) => {
                self.status = format!("Export serialization error: {}", e);
            }
        }
    }

    fn save_all(&mut self) {
        if let Err(e) = save_snippets(&self.snippets) {
            self.status = format!("Save error: {}", e);
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

    fn syntect_to_egui_color(color: SyntectColor) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(color.r, color.g, color.b, color.a)
    }

    fn syntax_layout_job(
        syntax_set: &SyntaxSet,
        theme: &Theme,
        language: &str,
        code: &str,
        wrap_width: f32,
    ) -> LayoutJob {
        let syntax = syntax_set
            .find_syntax_by_token(language)
            .or_else(|| syntax_set.find_syntax_by_extension(language))
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut job = LayoutJob::default();
        job.wrap.max_width = wrap_width;

        for line in LinesWithEndings::from(code) {
            match highlighter.highlight_line(line, syntax_set) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        let mut format = TextFormat {
                            font_id: egui::FontId::monospace(14.0),
                            color: Self::syntect_to_egui_color(style.foreground),
                            ..Default::default()
                        };

                        if style.font_style.contains(FontStyle::BOLD) {
                            format.font_id = egui::FontId::new(
                                14.0,
                                egui::FontFamily::Name("monospace".into()),
                            );
                        }

                        if style.font_style.contains(FontStyle::ITALIC) {
                            format.italics = true;
                        }

                        if style.font_style.contains(FontStyle::UNDERLINE) {
                            format.underline = egui::Stroke::new(
                                1.0,
                                Self::syntect_to_egui_color(style.foreground),
                            );
                        }

                        job.append(text, 0.0, format);
                    }
                }
                Err(_) => {
                    job.append(
                        line,
                        0.0,
                        TextFormat {
                            font_id: egui::FontId::monospace(14.0),
                            color: egui::Color32::LIGHT_GRAY,
                            ..Default::default()
                        },
                    );
                }
            }
        }

        job
    }

    fn code_preview(snippet: &Snippet) -> String {
        let code_preview = snippet
            .code
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim();

        if code_preview.len() > 45 {
            format!("{}...", &code_preview[..45])
        } else {
            code_preview.to_string()
        }
    }

    fn collect_all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .snippets
            .iter()
            .flat_map(|s| s.tags.iter().cloned())
            .collect();

        tags.sort_by_key(|t| t.to_lowercase());
        tags.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        tags
    }
}

impl eframe::App for SnippetApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.heading("Snippet Manager v0.0.4");
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

                if ui.button("Import").clicked() {
                    self.import_snippets();
                }

                if ui.button("Export").clicked() {
                    self.export_snippets();
                }
            });

            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
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

            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.label("Tag filter:");

                let all_tags = self.collect_all_tags();

                if ui
                    .selectable_label(self.active_tag.is_none(), "All")
                    .clicked()
                {
                    self.active_tag = None;
                }

                for tag in all_tags {
                    let selected = self
                        .active_tag
                        .as_ref()
                        .map(|t| t.eq_ignore_ascii_case(&tag))
                        .unwrap_or(false);

                    if ui.selectable_label(selected, tag.clone()).clicked() {
                        if selected {
                            self.active_tag = None;
                        } else {
                            self.active_tag = Some(tag);
                        }
                    }
                }
            });

            ui.add_space(2.0);
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Status: {}", self.status));
                ui.separator();
                ui.label(format!("Total snippets: {}", self.snippets.len()));
                ui.separator();
                ui.label(format!("Selected: {}", self.selected_snippet_label()));
                ui.separator();
                ui.label(format!(
                    "Active tag: {}",
                    self.active_tag.as_deref().unwrap_or("None")
                ));
            });
        });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Snippets");
                ui.separator();

                let filtered = self.filtered_indices();

                if filtered.is_empty() {
                    ui.label("No snippets found");
                    return;
                }

                let mut clicked_id: Option<Uuid> = None;
                let mut clicked_tag: Option<String> = None;

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

                        let code_preview = Self::code_preview(snippet);

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

                            if !snippet.tags.is_empty() {
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    for tag in &snippet.tags {
                                        if ui.small_button(format!("#{tag}")).clicked() {
                                            clicked_tag = Some(tag.clone());
                                        }
                                    }
                                });
                            }
                        });

                        ui.add_space(6.0);
                    }
                });

                if let Some(id) = clicked_id {
                    self.select_snippet(id);
                }

                if let Some(tag) = clicked_tag {
                    self.active_tag = Some(tag);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Editor");
            ui.separator();

            ui.label("Title");
            ui.add(egui::TextEdit::singleline(&mut self.title_input));

            ui.add_space(6.0);

            ui.label("Language");
            ui.add(
                egui::TextEdit::singleline(&mut self.language_input)
                    .hint_text("rust, cpp, js, py..."),
            );

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

            let syntax_set = &self.syntax_set;
            let theme = &self.theme;
            let language = self.language_input.clone();

            let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                let job = SnippetApp::syntax_layout_job(
                    syntax_set,
                    theme,
                    &language,
                    text,
                    wrap_width,
                );
                ui.fonts(|fonts| fonts.layout_job(job))
            };

            ui.add(
                egui::TextEdit::multiline(&mut self.code_input)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(22)
                    .desired_width(f32::INFINITY)
                    .layouter(&mut layouter),
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
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Snippet Manager v0.0.4",
        options,
        Box::new(|_cc| Ok(Box::new(SnippetApp::default()))),
    )
}