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
            title: "Untitled".to_string(),
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
    quick_search_open: bool,
    quick_search_query: String,
    quick_selected_index: usize,
    style_applied: bool,
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
            quick_search_open: false,
            quick_search_query: String::new(),
            quick_selected_index: 0,
            style_applied: false,
        };

        if let Some(first) = app.snippets.first().cloned() {
            app.selected_id = Some(first.id);
            app.load_into_editor(&first);
        }

        app
    }
}

impl SnippetApp {
    fn apply_notion_style(&mut self, ctx: &egui::Context) {
        if self.style_applied {
            return;
        }

        let mut style = (*ctx.style()).clone();

        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(16.0);
        style.spacing.menu_margin = egui::Margin::same(10.0);
        style.visuals = egui::Visuals::light();

        style.visuals.override_text_color = Some(egui::Color32::from_rgb(45, 45, 45));
        style.visuals.panel_fill = egui::Color32::from_rgb(251, 251, 249);
        style.visuals.window_fill = egui::Color32::from_rgb(255, 255, 255);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(245, 245, 242);
        style.visuals.faint_bg_color = egui::Color32::from_rgb(247, 247, 245);
        style.visuals.code_bg_color = egui::Color32::from_rgb(248, 248, 246);

        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(250, 250, 248);
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(232, 232, 228));

        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(255, 255, 255);
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(230, 230, 226));

        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(244, 244, 241);
        style.visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(219, 219, 214));

        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(238, 238, 234);
        style.visuals.widgets.active.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 210, 205));

        style.visuals.selection.bg_fill = egui::Color32::from_rgb(225, 232, 255);
        style.visuals.selection.stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(120, 150, 255));

        style.visuals.window_rounding = egui::Rounding::same(14.0);
        style.visuals.menu_rounding = egui::Rounding::same(12.0);

        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.open.rounding = egui::Rounding::same(10.0);

        ctx.set_style(style);
        self.style_applied = true;
    }

    fn filtered_indices(&self) -> Vec<usize> {
        self.filtered_indices_for_query(&self.search)
    }

    fn filtered_indices_for_query(&self, query: &str) -> Vec<usize> {
        let q = query.trim();
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

    fn quick_search_results(&self) -> Vec<usize> {
        self.filtered_indices_for_query(&self.quick_search_query)
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
            snippet.title = if new_title.is_empty() {
                "Untitled".to_string()
            } else {
                new_title
            };
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

    fn open_quick_search(&mut self) {
        self.quick_search_open = true;
        self.quick_search_query.clear();
        self.quick_selected_index = 0;
        self.status = "Quick search opened".to_string();
    }

    fn close_quick_search(&mut self) {
        self.quick_search_open = false;
        self.quick_selected_index = 0;
    }

    fn activate_quick_search_selection(&mut self) {
        let results = self.quick_search_results();

        if results.is_empty() {
            self.status = "No snippet found".to_string();
            return;
        }

        let selected = self.quick_selected_index.min(results.len().saturating_sub(1));
        let snippet_id = self.snippets[results[selected]].id;
        self.select_snippet(snippet_id);
        self.close_quick_search();
        self.status = "Snippet opened from quick search".to_string();
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut new_pressed = false;
        let mut save_pressed = false;
        let mut delete_pressed = false;
        let mut copy_pressed = false;
        let mut quick_search_pressed = false;

        ctx.input(|i| {
            new_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::N);
            save_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::S);
            delete_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::D);
            copy_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::C);
            quick_search_pressed =
                i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Space);
        });

        if quick_search_pressed {
            self.open_quick_search();
            return;
        }

        if self.quick_search_open {
            return;
        }

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

    fn handle_quick_search_keys(&mut self, ctx: &egui::Context, result_len: usize) {
        let mut escape_pressed = false;
        let mut enter_pressed = false;
        let mut arrow_down = false;
        let mut arrow_up = false;

        ctx.input(|i| {
            escape_pressed = i.key_pressed(egui::Key::Escape);
            enter_pressed = i.key_pressed(egui::Key::Enter);
            arrow_down = i.key_pressed(egui::Key::ArrowDown);
            arrow_up = i.key_pressed(egui::Key::ArrowUp);
        });

        if escape_pressed {
            self.close_quick_search();
            self.status = "Quick search closed".to_string();
            return;
        }

        if result_len == 0 {
            self.quick_selected_index = 0;
            return;
        }

        if arrow_down {
            self.quick_selected_index = (self.quick_selected_index + 1) % result_len;
        }

        if arrow_up {
            if self.quick_selected_index == 0 {
                self.quick_selected_index = result_len - 1;
            } else {
                self.quick_selected_index -= 1;
            }
        }

        if enter_pressed {
            self.activate_quick_search_selection();
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
                            format.font_id = egui::FontId::new(14.0, egui::FontFamily::Monospace);
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

        if code_preview.len() > 50 {
            format!("{}...", &code_preview[..50])
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

    fn tag_button(ui: &mut egui::Ui, label: &str, active: bool) -> egui::Response {
        let fill = if active {
            egui::Color32::from_rgb(229, 236, 255)
        } else {
            egui::Color32::from_rgb(244, 244, 241)
        };

        let stroke = if active {
            egui::Stroke::new(1.0, egui::Color32::from_rgb(135, 160, 255))
        } else {
            egui::Stroke::new(1.0, egui::Color32::from_rgb(228, 228, 223))
        };

        ui.add(
            egui::Button::new(egui::RichText::new(label).size(12.0))
                .fill(fill)
                .stroke(stroke)
                .rounding(egui::Rounding::same(999.0)),
        )
    }

    fn action_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(label)
                .fill(egui::Color32::from_rgb(255, 255, 255))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(226, 226, 221),
                ))
                .rounding(egui::Rounding::same(10.0)),
        )
    }

    fn secondary_text() -> egui::Color32 {
        egui::Color32::from_rgb(120, 120, 115)
    }
}

impl eframe::App for SnippetApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.apply_notion_style(ctx);
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("top_panel")
            .exact_height(92.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new("SnippetManager")
                                .size(24.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new("A cleaner Rust snippet workspace")
                                .size(13.0)
                                .color(Self::secondary_text()),
                        );
                    });

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            if Self::action_button(ui, "Quick Search").clicked() {
                                self.open_quick_search();
                            }

                            if Self::action_button(ui, "Export").clicked() {
                                self.export_snippets();
                            }

                            if Self::action_button(ui, "Import").clicked() {
                                self.import_snippets();
                            }

                            if Self::action_button(ui, "Delete").clicked() {
                                self.delete_current();
                            }

                            if Self::action_button(ui, "Save").clicked() {
                                self.save_current();
                            }

                            if Self::action_button(ui, "New").clicked() {
                                self.create_new_snippet();
                            }
                        },
                    );
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    ui.add_sized(
                        [280.0, 34.0],
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Search snippets..."),
                    );

                    ui.label(
                        egui::RichText::new("Ctrl+Shift+Space")
                            .size(12.0)
                            .color(Self::secondary_text()),
                    );
                    ui.label(
                        egui::RichText::new("opens quick search")
                            .size(12.0)
                            .color(Self::secondary_text()),
                    );
                });
            });

        egui::TopBottomPanel::bottom("bottom_panel")
            .exact_height(34.0)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!("Status: {}", self.status))
                            .size(12.0)
                            .color(Self::secondary_text()),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Snippets: {}", self.snippets.len()))
                            .size(12.0)
                            .color(Self::secondary_text()),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Selected: {}", self.selected_snippet_label()))
                            .size(12.0)
                            .color(Self::secondary_text()),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "Tag: {}",
                            self.active_tag.as_deref().unwrap_or("All")
                        ))
                        .size(12.0)
                        .color(Self::secondary_text()),
                    );
                });
            });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(247, 247, 244))
                    .inner_margin(egui::Margin::same(14.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Library")
                                    .size(18.0)
                                    .strong(),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{}", self.snippets.len()))
                                            .size(12.0)
                                            .color(Self::secondary_text()),
                                    );
                                },
                            );
                        });

                        ui.add_space(6.0);

                        ui.horizontal_wrapped(|ui| {
                            let all_selected = self.active_tag.is_none();
                            if Self::tag_button(ui, "All", all_selected).clicked() {
                                self.active_tag = None;
                            }

                            for tag in self.collect_all_tags() {
                                let selected = self
                                    .active_tag
                                    .as_ref()
                                    .map(|t| t.eq_ignore_ascii_case(&tag))
                                    .unwrap_or(false);

                                if Self::tag_button(ui, &tag, selected).clicked() {
                                    if selected {
                                        self.active_tag = None;
                                    } else {
                                        self.active_tag = Some(tag);
                                    }
                                }
                            }
                        });

                        ui.add_space(12.0);

                        let filtered = self.filtered_indices();

                        if filtered.is_empty() {
                            ui.add_space(20.0);
                            ui.label(
                                egui::RichText::new("No snippets found")
                                    .size(14.0)
                                    .color(Self::secondary_text()),
                            );
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

                                let language = if snippet.language.trim().is_empty() {
                                    "unknown"
                                } else {
                                    &snippet.language
                                };

                                let preview = Self::code_preview(snippet);

                                let fill = if selected {
                                    egui::Color32::from_rgb(235, 240, 255)
                                } else {
                                    egui::Color32::from_rgb(255, 255, 255)
                                };

                                let stroke = if selected {
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(145, 165, 255))
                                } else {
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(232, 232, 227))
                                };

                                egui::Frame::none()
                                    .fill(fill)
                                    .stroke(stroke)
                                    .rounding(egui::Rounding::same(14.0))
                                    .inner_margin(egui::Margin::same(12.0))
                                    .show(ui, |ui| {
                                        if ui
                                            .selectable_label(
                                                selected,
                                                egui::RichText::new(title).size(15.0).strong(),
                                            )
                                            .clicked()
                                        {
                                            clicked_id = Some(snippet.id);
                                        }

                                        ui.label(
                                            egui::RichText::new(language)
                                                .size(12.0)
                                                .color(Self::secondary_text()),
                                        );

                                        if !preview.is_empty() {
                                            ui.label(
                                                egui::RichText::new(preview)
                                                    .size(12.0)
                                                    .italics()
                                                    .color(Self::secondary_text()),
                                            );
                                        }

                                        if !snippet.tags.is_empty() {
                                            ui.add_space(4.0);
                                            ui.horizontal_wrapped(|ui| {
                                                for tag in &snippet.tags {
                                                    if Self::tag_button(ui, tag, false).clicked() {
                                                        clicked_tag = Some(tag.clone());
                                                    }
                                                }
                                            });
                                        }
                                    });

                                ui.add_space(8.0);
                            }
                        });

                        if let Some(id) = clicked_id {
                            self.select_snippet(id);
                        }

                        if let Some(tag) = clicked_tag {
                            self.active_tag = Some(tag);
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(255, 255, 255))
                .inner_margin(egui::Margin::same(18.0))
                .rounding(egui::Rounding::same(16.0))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(236, 236, 231),
                ))
                .show(ui, |ui| {
                    ui.add_space(2.0);

                    ui.label(
                        egui::RichText::new("Title")
                            .size(12.0)
                            .color(Self::secondary_text()),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.title_input)
                            .hint_text("Untitled snippet"),
                    );

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Language")
                                    .size(12.0)
                                    .color(Self::secondary_text()),
                            );
                            ui.add_sized(
                                [180.0, 32.0],
                                egui::TextEdit::singleline(&mut self.language_input)
                                    .hint_text("rust"),
                            );
                        });

                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("Tags")
                                    .size(12.0)
                                    .color(Self::secondary_text()),
                            );
                            ui.add_sized(
                                [320.0, 32.0],
                                egui::TextEdit::singleline(&mut self.tags_input)
                                    .hint_text("rust, async, algorithm"),
                            );
                        });
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Code")
                                .size(12.0)
                                .color(Self::secondary_text()),
                        );

                        if Self::action_button(ui, "Copy").clicked() {
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
                            .desired_rows(28)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter),
                    );
                });
        });

        if self.quick_search_open {
            let results = self.quick_search_results();

            if !results.is_empty() && self.quick_selected_index >= results.len() {
                self.quick_selected_index = results.len() - 1;
            }

            self.handle_quick_search_keys(ctx, results.len());

            egui::Window::new("Quick Snippet Search")
                .collapsible(false)
                .resizable(false)
                .default_width(560.0)
                .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("Jump to a snippet")
                            .size(20.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("Type title, language, tag, or code")
                            .size(12.0)
                            .color(Self::secondary_text()),
                    );

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.quick_search_query)
                            .hint_text("Search..."),
                    );
                    response.request_focus();

                    ui.add_space(8.0);

                    if results.is_empty() {
                        ui.label(
                            egui::RichText::new("No snippets found")
                                .size(13.0)
                                .color(Self::secondary_text()),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(280.0)
                            .show(ui, |ui| {
                                for (display_index, snippet_index) in results.iter().enumerate() {
                                    let snippet = self.snippets[*snippet_index].clone();
                                    let selected = display_index == self.quick_selected_index;

                                    let title = if snippet.title.trim().is_empty() {
                                        "Untitled".to_string()
                                    } else {
                                        snippet.title.clone()
                                    };

                                    let meta = format!(
                                        "{}  |  {}",
                                        if snippet.language.trim().is_empty() {
                                            "unknown".to_string()
                                        } else {
                                            snippet.language.clone()
                                        },
                                        Self::format_tags(&snippet.tags)
                                    );

                                    let preview = Self::code_preview(&snippet);

                                    let fill = if selected {
                                        egui::Color32::from_rgb(235, 240, 255)
                                    } else {
                                        egui::Color32::from_rgb(255, 255, 255)
                                    };

                                    egui::Frame::none()
                                        .fill(fill)
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(232, 232, 227),
                                        ))
                                        .rounding(egui::Rounding::same(12.0))
                                        .inner_margin(egui::Margin::same(12.0))
                                        .show(ui, |ui| {
                                            if ui
                                                .selectable_label(
                                                    selected,
                                                    egui::RichText::new(title.clone())
                                                        .size(14.0)
                                                        .strong(),
                                                )
                                                .clicked()
                                            {
                                                self.quick_selected_index = display_index;
                                                self.activate_quick_search_selection();
                                            }

                                            ui.label(
                                                egui::RichText::new(meta.clone())
                                                    .size(12.0)
                                                    .color(Self::secondary_text()),
                                            );

                                            if !preview.is_empty() {
                                                ui.label(
                                                    egui::RichText::new(preview.clone())
                                                        .size(12.0)
                                                        .italics()
                                                        .color(Self::secondary_text()),
                                                );
                                            }
                                        });

                                    ui.add_space(6.0);
                                }
                            });
                    }

                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("↑ ↓ navigate   •   Enter open   •   Esc close")
                                .size(12.0)
                                .color(Self::secondary_text()),
                        );
                    });
                });
        }
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
        viewport: egui::ViewportBuilder::default().with_inner_size([1380.0, 860.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Snippet Manager v0.0.6",
        options,
        Box::new(|_cc| Ok(Box::new(SnippetApp::default()))),
    )
}