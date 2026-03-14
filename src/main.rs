use arboard::Clipboard;
use eframe::egui;
use eframe::egui::text::{LayoutJob, TextFormat};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color as SyntectColor, FontStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use uuid::Uuid;

const FILE_PATH: &str = "snippets.json";
const STATE_PATH: &str = "snippet_manager_state.json";

#[derive(Serialize, Deserialize, Clone)]
struct Snippet {
    id: Uuid,
    title: String,
    language: String,
    tags: Vec<String>,
    code: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_folder")]
    folder: String,
    #[serde(default)]
    favorite: bool,
    #[serde(default)]
    created_at: u64,
    #[serde(default)]
    updated_at: u64,
}

fn default_folder() -> String {
    "General".to_string()
}

fn now_ts() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    }
}

impl Snippet {
    fn new() -> Self {
        let now = now_ts();
        Self {
            id: Uuid::new_v4(),
            title: "New Snippet".to_string(),
            language: "Rust".to_string(),
            tags: vec![],
            code: String::new(),
            description: String::new(),
            folder: "General".to_string(),
            favorite: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct AppState {
    folders: Vec<String>,
    recent_ids: Vec<Uuid>,
}

#[derive(PartialEq, Eq, Clone)]
enum SidebarFilter {
    All,
    Folder(String),
    Language(String),
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum SortMode {
    UpdatedDesc,
    TitleAsc,
    LanguageAsc,
    FavoritesFirst,
}

#[derive(Clone, Copy)]
enum CopyMode {
    Raw,
    Markdown,
}

struct SnippetApp {
    snippets: Vec<Snippet>,
    selected_id: Option<Uuid>,

    search: String,
    status: String,

    title_input: String,
    description_input: String,
    folder_input: String,
    language_input: String,
    tags_input: String,
    code_input: String,

    matcher: SkimMatcherV2,
    syntax_set: SyntaxSet,
    theme: Theme,

    quick_search_open: bool,
    quick_search_query: String,
    quick_selected_index: usize,

    style_applied: bool,
    sidebar_filter: SidebarFilter,

    folders: Vec<String>,
    new_folder_open: bool,
    new_folder_name: String,

    edit_mode: bool,
    sort_mode: SortMode,
    recent_ids: Vec<Uuid>,

    template_open: bool,
    template_fields: Vec<String>,
    template_values: BTreeMap<String, String>,
    template_copy_mode: CopyMode,
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
        let app_state = load_app_state();

        let mut folders = app_state.folders;
        if folders.is_empty() {
            folders.push("General".to_string());
        }

        let mut app = Self {
            snippets,
            selected_id: None,

            search: String::new(),
            status: "Ready".to_string(),

            title_input: String::new(),
            description_input: String::new(),
            folder_input: String::new(),
            language_input: String::new(),
            tags_input: String::new(),
            code_input: String::new(),

            matcher: SkimMatcherV2::default(),
            syntax_set,
            theme,

            quick_search_open: false,
            quick_search_query: String::new(),
            quick_selected_index: 0,

            style_applied: false,
            sidebar_filter: SidebarFilter::All,

            folders,
            new_folder_open: false,
            new_folder_name: String::new(),

            edit_mode: false,
            sort_mode: SortMode::FavoritesFirst,
            recent_ids: app_state.recent_ids,

            template_open: false,
            template_fields: Vec::new(),
            template_values: BTreeMap::new(),
            template_copy_mode: CopyMode::Raw,
        };

        app.ensure_default_folders();

        if let Some(first) = app.snippets.first().cloned() {
            app.selected_id = Some(first.id);
            app.load_into_editor(&first);
        } else {
            app.folder_input = "General".to_string();
            app.language_input = "Rust".to_string();
            app.edit_mode = true;
        }

        app
    }
}

impl SnippetApp {
    fn apply_dark_vault_style(&mut self, ctx: &egui::Context) {
        if self.style_applied {
            return;
        }

        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();

        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style.spacing.window_margin = egui::Margin::same(12.0);

        style.visuals.override_text_color = Some(egui::Color32::from_rgb(235, 240, 255));
        style.visuals.panel_fill = egui::Color32::from_rgb(8, 16, 35);
        style.visuals.window_fill = egui::Color32::from_rgb(10, 18, 40);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(13, 24, 52);
        style.visuals.faint_bg_color = egui::Color32::from_rgb(16, 28, 58);
        style.visuals.code_bg_color = egui::Color32::from_rgb(11, 22, 48);

        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(12, 22, 46);
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(30, 47, 84));

        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(14, 25, 52);
        style.visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(31, 47, 84));

        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(21, 37, 74);
        style.visuals.widgets.hovered.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(69, 98, 164));

        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(39, 57, 108);
        style.visuals.widgets.active.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(112, 136, 255));

        style.visuals.selection.bg_fill = egui::Color32::from_rgb(57, 83, 180);
        style.visuals.selection.stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(168, 189, 255));

        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.open.rounding = egui::Rounding::same(10.0);
        style.visuals.window_rounding = egui::Rounding::same(14.0);

        ctx.set_style(style);
        self.style_applied = true;
    }

    fn ensure_default_folders(&mut self) {
        let mut all_folders = self.folders.clone();

        if !all_folders.iter().any(|f| f.eq_ignore_ascii_case("General")) {
            all_folders.push("General".to_string());
        }

        for snippet in &self.snippets {
            let folder = if snippet.folder.trim().is_empty() {
                "General".to_string()
            } else {
                snippet.folder.clone()
            };

            if !all_folders.iter().any(|f| f.eq_ignore_ascii_case(&folder)) {
                all_folders.push(folder);
            }
        }

        all_folders.sort_by_key(|f| f.to_lowercase());
        all_folders.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        self.folders = all_folders;
    }

    fn persist_state(&mut self) {
        self.ensure_default_folders();

        let state = AppState {
            folders: self.folders.clone(),
            recent_ids: self.recent_ids.clone(),
        };

        if let Err(e) = save_app_state(&state) {
            self.status = format!("State save error: {}", e);
        }
    }

    fn load_into_editor(&mut self, snippet: &Snippet) {
        self.title_input = snippet.title.clone();
        self.description_input = snippet.description.clone();
        self.folder_input = if snippet.folder.trim().is_empty() {
            "General".to_string()
        } else {
            snippet.folder.clone()
        };
        self.language_input = if snippet.language.trim().is_empty() {
            "Text".to_string()
        } else {
            snippet.language.clone()
        };
        self.tags_input = snippet.tags.join(", ");
        self.code_input = snippet.code.clone();

        if !self
            .folders
            .iter()
            .any(|f| f.eq_ignore_ascii_case(&self.folder_input))
        {
            self.folders.push(self.folder_input.clone());
            self.persist_state();
        }
    }

    fn clear_editor(&mut self) {
        self.selected_id = None;
        self.title_input.clear();
        self.description_input.clear();
        self.folder_input = "General".to_string();
        self.language_input = "Rust".to_string();
        self.tags_input.clear();
        self.code_input.clear();
    }

    fn create_new_snippet(&mut self) {
        let mut snippet = Snippet::new();
        if !self.folder_input.trim().is_empty() {
            snippet.folder = self.folder_input.trim().to_string();
        }

        self.selected_id = Some(snippet.id);
        self.load_into_editor(&snippet);
        self.snippets.push(snippet);
        self.edit_mode = true;
        self.status = "New snippet created".to_string();
        self.save_all();
    }

    fn duplicate_current(&mut self) {
        let Some(id) = self.selected_id else {
            self.status = "No snippet selected".to_string();
            return;
        };

        let original = match self.snippets.iter().find(|s| s.id == id).cloned() {
            Some(s) => s,
            None => {
                self.status = "Snippet not found".to_string();
                return;
            }
        };

        let now = now_ts();
        let mut duplicate = original.clone();
        duplicate.id = Uuid::new_v4();
        duplicate.title = format!("{} Copy", original.title);
        duplicate.created_at = now;
        duplicate.updated_at = now;

        self.snippets.push(duplicate.clone());
        self.selected_id = Some(duplicate.id);
        self.load_into_editor(&duplicate);
        self.edit_mode = true;
        self.status = "Snippet duplicated".to_string();
        self.save_all();
    }

    fn toggle_favorite_current(&mut self) {
        let Some(id) = self.selected_id else {
            self.status = "No snippet selected".to_string();
            return;
        };

        if let Some(snippet) = self.snippets.iter_mut().find(|s| s.id == id) {
            snippet.favorite = !snippet.favorite;
            snippet.updated_at = now_ts();
            self.status = if snippet.favorite {
                "Snippet pinned".to_string()
            } else {
                "Snippet unpinned".to_string()
            };
            self.save_all();
        }
    }

    fn create_folder(&mut self) {
        let folder_name = self.new_folder_name.trim().to_string();

        if folder_name.is_empty() {
            self.status = "Folder name cannot be empty".to_string();
            return;
        }

        if self
            .folders
            .iter()
            .any(|f| f.eq_ignore_ascii_case(&folder_name))
        {
            self.status = "Folder already exists".to_string();
            return;
        }

        self.folders.push(folder_name.clone());
        self.folders.sort_by_key(|f| f.to_lowercase());
        self.folder_input = folder_name.clone();
        self.sidebar_filter = SidebarFilter::Folder(folder_name.clone());
        self.new_folder_name.clear();
        self.new_folder_open = false;
        self.persist_state();
        self.status = format!("Folder '{}' created", folder_name);
    }

    fn save_current(&mut self) {
        let Some(id) = self.selected_id else {
            self.status = "No snippet selected".to_string();
            return;
        };

        let new_title = self.title_input.trim().to_string();
        let new_description = self.description_input.trim().to_string();

        let new_folder = if self.folder_input.trim().is_empty() {
            "General".to_string()
        } else {
            self.folder_input.trim().to_string()
        };

        let new_language = if self.language_input.trim().is_empty() {
            "Text".to_string()
        } else {
            self.language_input.trim().to_string()
        };

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
            snippet.description = new_description;
            snippet.folder = new_folder.clone();
            snippet.language = new_language;
            snippet.tags = new_tags;
            snippet.code = new_code;
            snippet.updated_at = now_ts();

            if !self.folders.iter().any(|f| f.eq_ignore_ascii_case(&new_folder)) {
                self.folders.push(new_folder);
            }

            self.persist_state();
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
        self.recent_ids.retain(|x| *x != id);

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

        self.persist_state();
        self.save_all();
        self.status = "Snippet deleted".to_string();
    }

    fn copy_text(&mut self, text: String, message: &str) {
        match Clipboard::new() {
            Ok(mut clipboard) => match clipboard.set_text(text) {
                Ok(_) => self.status = message.to_string(),
                Err(e) => self.status = format!("Clipboard error: {}", e),
            },
            Err(e) => self.status = format!("Clipboard unavailable: {}", e),
        }
    }

    fn copy_code_raw(&mut self) {
        if self.code_input.trim().is_empty() {
            self.status = "No code to copy".to_string();
            return;
        }
        self.copy_text(self.code_input.clone(), "Code copied to clipboard");
    }

    fn copy_code_markdown(&mut self) {
        if self.code_input.trim().is_empty() {
            self.status = "No code to copy".to_string();
            return;
        }

        let lang = self.language_input.trim().to_lowercase();
        let markdown = format!("```{}\n{}\n```", lang, self.code_input);
        self.copy_text(markdown, "Markdown code block copied");
    }

    fn open_template_copy(&mut self, mode: CopyMode) {
        let fields = extract_placeholders(&self.code_input);

        if fields.is_empty() {
            match mode {
                CopyMode::Raw => self.copy_code_raw(),
                CopyMode::Markdown => self.copy_code_markdown(),
            }
            return;
        }

        self.template_fields = fields.clone();
        self.template_values.clear();

        for field in fields {
            self.template_values.insert(field, String::new());
        }

        self.template_copy_mode = mode;
        self.template_open = true;
        self.status = "Fill template values".to_string();
    }

    fn confirm_template_copy(&mut self) {
        let rendered = render_template(&self.code_input, &self.template_values);

        match self.template_copy_mode {
            CopyMode::Raw => {
                self.copy_text(rendered, "Rendered template copied");
            }
            CopyMode::Markdown => {
                let lang = self.language_input.trim().to_lowercase();
                let wrapped = format!("```{}\n{}\n```", lang, rendered);
                self.copy_text(wrapped, "Rendered Markdown copied");
            }
        }

        self.template_open = false;
        self.template_fields.clear();
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

                        if !self
                            .folders
                            .iter()
                            .any(|f| f.eq_ignore_ascii_case(&snippet.folder))
                        {
                            self.folders.push(snippet.folder.clone());
                        }

                        self.snippets.push(snippet);
                        added += 1;
                    }

                    self.persist_state();
                    self.save_all();

                    self.status = if added == 0 {
                        "No new snippets were imported".to_string()
                    } else {
                        format!("Imported {} snippet(s)", added)
                    };
                }
                Err(e) => self.status = format!("Import parse error: {}", e),
            },
            Err(e) => self.status = format!("Import read error: {}", e),
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
                Ok(_) => self.status = "Snippets exported".to_string(),
                Err(e) => self.status = format!("Export write error: {}", e),
            },
            Err(e) => self.status = format!("Export serialization error: {}", e),
        }
    }

    fn save_all(&mut self) {
        if let Err(e) = save_snippets(&self.snippets) {
            self.status = format!("Save error: {}", e);
        }
    }

    fn record_recent(&mut self, id: Uuid) {
        self.recent_ids.retain(|x| *x != id);
        self.recent_ids.insert(0, id);
        self.recent_ids.truncate(12);
        self.persist_state();
    }

    fn select_snippet(&mut self, id: Uuid) {
        self.selected_id = Some(id);

        if let Some(snippet) = self.snippets.iter().find(|s| s.id == id).cloned() {
            self.load_into_editor(&snippet);
            self.record_recent(id);
            self.status = "Snippet selected".to_string();
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
        let mut toggle_edit_pressed = false;

        ctx.input(|i| {
            new_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::N);
            save_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::S);
            delete_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::D);
            copy_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::C);
            quick_search_pressed =
                i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Space);
            toggle_edit_pressed = i.modifiers.ctrl && i.key_pressed(egui::Key::E);
        });

        if quick_search_pressed {
            self.open_quick_search();
            return;
        }

        if self.quick_search_open || self.template_open || self.new_folder_open {
            return;
        }

        if toggle_edit_pressed {
            self.edit_mode = !self.edit_mode;
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
            self.copy_code_raw();
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

    fn matches_sidebar_filter(&self, snippet: &Snippet) -> bool {
        match &self.sidebar_filter {
            SidebarFilter::All => true,
            SidebarFilter::Folder(folder) => snippet.folder.eq_ignore_ascii_case(folder),
            SidebarFilter::Language(language) => snippet.language.eq_ignore_ascii_case(language),
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        self.filtered_indices_for_query(&self.search)
    }

    fn sort_indices(&self, items: &mut [usize]) {
        items.sort_by(|a, b| {
            let sa = &self.snippets[*a];
            let sb = &self.snippets[*b];

            match self.sort_mode {
                SortMode::UpdatedDesc => sb
                    .updated_at
                    .cmp(&sa.updated_at)
                    .then_with(|| sa.title.to_lowercase().cmp(&sb.title.to_lowercase())),
                SortMode::TitleAsc => sa
                    .title
                    .to_lowercase()
                    .cmp(&sb.title.to_lowercase())
                    .then_with(|| sb.updated_at.cmp(&sa.updated_at)),
                SortMode::LanguageAsc => sa
                    .language
                    .to_lowercase()
                    .cmp(&sb.language.to_lowercase())
                    .then_with(|| sa.title.to_lowercase().cmp(&sb.title.to_lowercase())),
                SortMode::FavoritesFirst => sb
                    .favorite
                    .cmp(&sa.favorite)
                    .then_with(|| sb.updated_at.cmp(&sa.updated_at))
                    .then_with(|| sa.title.to_lowercase().cmp(&sb.title.to_lowercase())),
            }
        });
    }

    fn filtered_indices_for_query(&self, query: &str) -> Vec<usize> {
        let q = query.trim();

        if q.is_empty() {
            let mut indices: Vec<usize> = self
                .snippets
                .iter()
                .enumerate()
                .filter(|(_, s)| self.matches_sidebar_filter(s))
                .map(|(i, _)| i)
                .collect();
            self.sort_indices(&mut indices);
            return indices;
        }

        let mut scored: Vec<(usize, i64)> = self
            .snippets
            .iter()
            .enumerate()
            .filter_map(|(i, snippet)| {
                if !self.matches_sidebar_filter(snippet) {
                    return None;
                }

                let tags = snippet.tags.join(" ");
                let haystack = format!(
                    "{} {} {} {} {} {}",
                    snippet.title,
                    snippet.description,
                    snippet.folder,
                    snippet.language,
                    tags,
                    snippet.code
                );

                self.matcher.fuzzy_match(&haystack, q).map(|score| (i, score))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));

        let mut indices: Vec<usize> = scored.into_iter().map(|(i, _)| i).collect();
        if indices.len() > 1 {
            self.sort_indices(&mut indices);
        }
        indices
    }

    fn quick_search_results(&self) -> Vec<usize> {
        let q = self.quick_search_query.trim();

        if q.is_empty() {
            let mut result = Vec::new();
            let mut seen = BTreeSet::new();

            for id in &self.recent_ids {
                if let Some((idx, _)) = self.snippets.iter().enumerate().find(|(_, s)| s.id == *id) {
                    if seen.insert(idx) {
                        result.push(idx);
                    }
                }
            }

            let mut rest: Vec<usize> = (0..self.snippets.len())
                .filter(|i| !seen.contains(i))
                .collect();
            self.sort_indices(&mut rest);
            result.extend(rest);
            return result;
        }

        let mut scored: Vec<(usize, i64)> = self
            .snippets
            .iter()
            .enumerate()
            .filter_map(|(i, snippet)| {
                let haystack = format!(
                    "{} {} {} {} {} {}",
                    snippet.title,
                    snippet.description,
                    snippet.folder,
                    snippet.language,
                    snippet.tags.join(" "),
                    snippet.code
                );

                self.matcher.fuzzy_match(&haystack, q).map(|score| (i, score))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.into_iter().map(|(i, _)| i).collect()
    }

    fn folder_counts(&self) -> Vec<(String, usize)> {
        let mut map: BTreeMap<String, usize> = BTreeMap::new();

        for folder in &self.folders {
            map.entry(folder.clone()).or_insert(0);
        }

        for snippet in &self.snippets {
            let folder = if snippet.folder.trim().is_empty() {
                "General".to_string()
            } else {
                snippet.folder.clone()
            };
            *map.entry(folder).or_insert(0) += 1;
        }

        map.into_iter().collect()
    }

    fn language_counts(&self) -> Vec<(String, usize)> {
        let mut map: BTreeMap<String, usize> = BTreeMap::new();

        for snippet in &self.snippets {
            let language = if snippet.language.trim().is_empty() {
                "Text".to_string()
            } else {
                snippet.language.clone()
            };
            *map.entry(language).or_insert(0) += 1;
        }

        map.into_iter().collect()
    }

    fn selected_snippet_label(&self) -> String {
        match self.selected_id {
            Some(id) => self
                .snippets
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.title.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            None => "None".to_string(),
        }
    }

    fn code_preview(snippet: &Snippet) -> String {
        if !snippet.description.trim().is_empty() {
            return snippet.description.clone();
        }

        let line = snippet
            .code
            .lines()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("")
            .trim();

        if line.len() > 64 {
            format!("{}...", &line[..64])
        } else {
            line.to_string()
        }
    }

    fn line_count(&self) -> usize {
        let count = self.code_input.lines().count();
        count.max(1)
    }

    fn line_numbers_text(&self) -> String {
        (1..=self.line_count())
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n")
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

    fn action_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(label)
                .fill(egui::Color32::from_rgb(24, 38, 72))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(46, 67, 115),
                ))
                .rounding(egui::Rounding::same(10.0)),
        )
    }

    fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(label)
                .fill(egui::Color32::from_rgb(92, 96, 255))
                .stroke(egui::Stroke::NONE)
                .rounding(egui::Rounding::same(10.0)),
        )
    }

    fn tag_chip(ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(
                egui::RichText::new(label)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(156, 179, 228)),
            )
            .fill(egui::Color32::from_rgb(32, 47, 84))
            .stroke(egui::Stroke::new(
                1.0,
                egui::Color32::from_rgb(48, 65, 108),
            ))
            .rounding(egui::Rounding::same(8.0)),
        )
    }

    fn muted() -> egui::Color32 {
        egui::Color32::from_rgb(128, 150, 195)
    }

    fn panel_bg() -> egui::Color32 {
        egui::Color32::from_rgb(9, 19, 42)
    }

    fn card_bg(selected: bool) -> egui::Color32 {
        if selected {
            egui::Color32::from_rgb(34, 51, 94)
        } else {
            egui::Color32::from_rgb(10, 21, 46)
        }
    }

    fn sort_label(mode: SortMode) -> &'static str {
        match mode {
            SortMode::UpdatedDesc => "Updated",
            SortMode::TitleAsc => "Title",
            SortMode::LanguageAsc => "Language",
            SortMode::FavoritesFirst => "Pinned",
        }
    }

    fn draw_view_mode(&self, ui: &mut egui::Ui) {
        let code = self.code_input.clone();
        let line_numbers = self.line_numbers_text();

        ui.horizontal(|ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(8, 17, 36))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(28, 43, 78),
                ))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.add_sized(
                        [42.0, 540.0],
                        egui::Label::new(
                            egui::RichText::new(line_numbers)
                                .monospace()
                                .size(13.0)
                                .color(Self::muted()),
                        ),
                    );
                });

            let syntax_set = &self.syntax_set;
            let theme = &self.theme;
            let language = self.language_input.clone();
            let job =
                Self::syntax_layout_job(syntax_set, theme, &language, &code, ui.available_width());

            egui::Frame::none()
                .fill(egui::Color32::from_rgb(8, 17, 36))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(28, 43, 78),
                ))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(540.0).show(ui, |ui| {
                        ui.label(job);
                    });
                });
        });
    }

    fn draw_edit_mode(&mut self, ui: &mut egui::Ui) {
        let line_numbers = self.line_numbers_text();

        ui.horizontal(|ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(8, 17, 36))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(28, 43, 78),
                ))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.add_sized(
                        [42.0, 540.0],
                        egui::Label::new(
                            egui::RichText::new(line_numbers)
                                .monospace()
                                .size(13.0)
                                .color(Self::muted()),
                        ),
                    );
                });

            let syntax_set = &self.syntax_set;
            let theme = &self.theme;
            let language = self.language_input.clone();

            let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                let job =
                    SnippetApp::syntax_layout_job(syntax_set, theme, &language, text, wrap_width);
                ui.fonts(|fonts| fonts.layout_job(job))
            };

            egui::Frame::none()
                .fill(egui::Color32::from_rgb(8, 17, 36))
                .stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgb(28, 43, 78),
                ))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.code_input)
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(28)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter),
                    );
                });
        });
    }
}

impl eframe::App for SnippetApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.apply_dark_vault_style(ctx);
        self.handle_shortcuts(ctx);

        egui::SidePanel::left("nav_panel")
            .resizable(false)
            .exact_width(196.0)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(6, 14, 30))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("</>").strong().size(18.0));
                            ui.label(
                                egui::RichText::new("SnippetVault")
                                    .size(22.0)
                                    .strong(),
                            );
                        });

                        ui.add_space(14.0);

                        if Self::primary_button(ui, "+  New Snippet").clicked() {
                            self.create_new_snippet();
                        }

                        if Self::action_button(ui, "+  New Folder").clicked() {
                            self.new_folder_open = true;
                        }

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("FOLDERS")
                                .size(11.0)
                                .color(Self::muted())
                                .strong(),
                        );

                        ui.add_space(6.0);

                        let all_selected = self.sidebar_filter == SidebarFilter::All;
                        if ui
                            .selectable_label(all_selected, "📁  All Snippets")
                            .clicked()
                        {
                            self.sidebar_filter = SidebarFilter::All;
                        }

                        for (folder, count) in self.folder_counts() {
                            ui.horizontal(|ui| {
                                let selected =
                                    self.sidebar_filter == SidebarFilter::Folder(folder.clone());

                                if ui
                                    .selectable_label(selected, format!("📁  {}", folder))
                                    .clicked()
                                {
                                    self.sidebar_filter = SidebarFilter::Folder(folder.clone());
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(count.to_string())
                                                .size(11.0)
                                                .color(Self::muted()),
                                        );
                                    },
                                );
                            });
                        }

                        ui.add_space(16.0);

                        ui.label(
                            egui::RichText::new("LANGUAGES")
                                .size(11.0)
                                .color(Self::muted())
                                .strong(),
                        );

                        ui.add_space(6.0);

                        for (language, count) in self.language_counts() {
                            ui.horizontal(|ui| {
                                let selected =
                                    self.sidebar_filter == SidebarFilter::Language(language.clone());

                                if ui
                                    .selectable_label(selected, format!("#  {}", language))
                                    .clicked()
                                {
                                    self.sidebar_filter = SidebarFilter::Language(language.clone());
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            egui::RichText::new(count.to_string())
                                                .size(11.0)
                                                .color(Self::muted()),
                                        );
                                    },
                                );
                            });
                        }
                    });
            });

        egui::SidePanel::left("list_panel")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(Self::panel_bg())
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [210.0, 34.0],
                                egui::TextEdit::singleline(&mut self.search)
                                    .hint_text("Search snippets..."),
                            );

                            egui::ComboBox::from_id_source("sort_mode")
                                .selected_text(Self::sort_label(self.sort_mode))
                                .width(90.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.sort_mode,
                                        SortMode::FavoritesFirst,
                                        "Pinned",
                                    );
                                    ui.selectable_value(
                                        &mut self.sort_mode,
                                        SortMode::UpdatedDesc,
                                        "Updated",
                                    );
                                    ui.selectable_value(
                                        &mut self.sort_mode,
                                        SortMode::TitleAsc,
                                        "Title",
                                    );
                                    ui.selectable_value(
                                        &mut self.sort_mode,
                                        SortMode::LanguageAsc,
                                        "Language",
                                    );
                                });
                        });

                        ui.add_space(10.0);

                        let filtered = self.filtered_indices();

                        if filtered.is_empty() {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(10, 21, 46))
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    egui::Color32::from_rgb(28, 43, 78),
                                ))
                                .inner_margin(egui::Margin::same(16.0))
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("No snippets found")
                                            .size(16.0)
                                            .strong(),
                                    );
                                    ui.label(
                                        egui::RichText::new(
                                            "Try a different search, switch filters, or create a new snippet.",
                                        )
                                        .size(12.0)
                                        .color(Self::muted()),
                                    );
                                    ui.add_space(8.0);
                                    if Self::primary_button(ui, "Create Snippet").clicked() {
                                        self.create_new_snippet();
                                    }
                                });
                            return;
                        }

                        let mut clicked_id: Option<Uuid> = None;
                        let snippets_snapshot = self.snippets.clone();

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for index in filtered {
                                let snippet = &snippets_snapshot[index];
                                let selected = self.selected_id == Some(snippet.id);

                                let title = if snippet.title.trim().is_empty() {
                                    "Untitled"
                                } else {
                                    &snippet.title
                                };

                                let preview = Self::code_preview(snippet);

                                egui::Frame::none()
                                    .fill(Self::card_bg(selected))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        if selected {
                                            egui::Color32::from_rgb(72, 96, 170)
                                        } else {
                                            egui::Color32::from_rgb(28, 43, 78)
                                        },
                                    ))
                                    .inner_margin(egui::Margin::same(12.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let prefix = if snippet.favorite { "★ " } else { "" };
                                            let title_resp = ui.selectable_label(
                                                selected,
                                                egui::RichText::new(format!("{}{}", prefix, title))
                                                    .strong()
                                                    .size(15.0),
                                            );
                                            if title_resp.clicked() {
                                                clicked_id = Some(snippet.id);
                                            }

                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        egui::RichText::new(&snippet.language)
                                                            .size(11.0)
                                                            .color(egui::Color32::from_rgb(
                                                                105, 147, 255,
                                                            ))
                                                            .strong(),
                                                    );
                                                },
                                            );
                                        });

                                        if !preview.is_empty() {
                                            ui.label(
                                                egui::RichText::new(preview)
                                                    .size(12.0)
                                                    .color(Self::muted()),
                                            );
                                        }

                                        if !snippet.tags.is_empty() {
                                            ui.add_space(6.0);
                                            ui.horizontal_wrapped(|ui| {
                                                for tag in &snippet.tags {
                                                    let _ = Self::tag_chip(ui, tag);
                                                }
                                            });
                                        }
                                    });

                                ui.add_space(6.0);
                            }
                        });

                        if let Some(id) = clicked_id {
                            self.select_snippet(id);
                            self.edit_mode = false;
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::none()
                .fill(egui::Color32::from_rgb(7, 18, 40))
                .inner_margin(egui::Margin::same(0.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let title = if self.title_input.trim().is_empty() {
                                "Untitled".to_string()
                            } else {
                                self.title_input.clone()
                            };

                            ui.label(egui::RichText::new(title).size(28.0).strong());

                            if !self.description_input.trim().is_empty() {
                                ui.label(
                                    egui::RichText::new(self.description_input.clone())
                                        .size(13.0)
                                        .color(Self::muted()),
                                );
                            }
                        });

                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let mode_label = if self.edit_mode { "View" } else { "Edit" };
                                if Self::primary_button(ui, mode_label).clicked() {
                                    self.edit_mode = !self.edit_mode;
                                }

                                if Self::action_button(ui, "★").clicked() {
                                    self.toggle_favorite_current();
                                }
                                if Self::action_button(ui, "Duplicate").clicked() {
                                    self.duplicate_current();
                                }
                                if Self::action_button(ui, "Copy MD").clicked() {
                                    self.copy_code_markdown();
                                }
                                if Self::action_button(ui, "Copy").clicked() {
                                    self.copy_code_raw();
                                }
                                if Self::action_button(ui, "Smart Copy").clicked() {
                                    self.open_template_copy(CopyMode::Raw);
                                }
                                if Self::action_button(ui, "Save").clicked() {
                                    self.save_current();
                                }
                            },
                        );
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(10.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(9, 19, 42))
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgb(28, 43, 78),
                        ))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Folder: {}", self.folder_input))
                                        .size(12.0)
                                        .color(Self::muted()),
                                );
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Language: {}",
                                        self.language_input
                                    ))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(105, 147, 255)),
                                );
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(format!("Lines: {}", self.line_count()))
                                        .size(12.0)
                                        .color(Self::muted()),
                                );
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(if self.edit_mode {
                                        "Mode: Edit"
                                    } else {
                                        "Mode: View"
                                    })
                                    .size(12.0)
                                    .color(Self::muted()),
                                );
                                ui.separator();
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Selected: {}",
                                        self.selected_snippet_label()
                                    ))
                                    .size(12.0)
                                    .color(Self::muted()),
                                );

                                for tag in self
                                    .tags_input
                                    .split(',')
                                    .map(|t| t.trim())
                                    .filter(|t| !t.is_empty())
                                {
                                    ui.separator();
                                    let _ = Self::tag_chip(ui, tag);
                                }
                            });
                        });

                    ui.add_space(10.0);

                    if self.edit_mode {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [260.0, 30.0],
                                egui::TextEdit::singleline(&mut self.title_input)
                                    .hint_text("Snippet title"),
                            );

                            ui.add_sized(
                                [160.0, 30.0],
                                egui::TextEdit::singleline(&mut self.language_input)
                                    .hint_text("Language"),
                            );

                            let folders = self.folders.clone();
                            egui::ComboBox::from_id_source("folder_combo")
                                .selected_text(if self.folder_input.trim().is_empty() {
                                    "General".to_string()
                                } else {
                                    self.folder_input.clone()
                                })
                                .width(170.0)
                                .show_ui(ui, |ui| {
                                    for folder in folders {
                                        ui.selectable_value(
                                            &mut self.folder_input,
                                            folder.clone(),
                                            folder,
                                        );
                                    }
                                });

                            if Self::action_button(ui, "New Folder").clicked() {
                                self.new_folder_open = true;
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [420.0, 30.0],
                                egui::TextEdit::singleline(&mut self.description_input)
                                    .hint_text("Short description"),
                            );
                            ui.add_sized(
                                [320.0, 30.0],
                                egui::TextEdit::singleline(&mut self.tags_input)
                                    .hint_text("tags, separated, by, commas"),
                            );
                        });

                        ui.add_space(10.0);
                        self.draw_edit_mode(ui);
                    } else {
                        self.draw_view_mode(ui);
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new(self.status.clone())
                                .size(12.0)
                                .color(Self::muted()),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Ctrl+E")
                                .size(12.0)
                                .color(Self::muted()),
                        );
                        ui.label(
                            egui::RichText::new("toggle view/edit")
                                .size(12.0)
                                .color(Self::muted()),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("Templates: use ${name} in code")
                                .size(12.0)
                                .color(Self::muted()),
                        );
                    });
                });
        });

        if self.quick_search_open {
            let results = self.quick_search_results();

            if !results.is_empty() && self.quick_selected_index >= results.len() {
                self.quick_selected_index = results.len() - 1;
            }

            self.handle_quick_search_keys(ctx, results.len());

            egui::Window::new("Quick Search")
                .collapsible(false)
                .resizable(false)
                .default_width(520.0)
                .anchor(egui::Align2::CENTER_TOP, [0.0, 60.0])
                .show(ctx, |ui| {
                    ui.add_sized(
                        [ui.available_width(), 34.0],
                        egui::TextEdit::singleline(&mut self.quick_search_query)
                            .hint_text("Search all snippets..."),
                    );

                    ui.add_space(8.0);

                    if self.quick_search_query.trim().is_empty() {
                        ui.label(
                            egui::RichText::new("Recent")
                                .size(12.0)
                                .color(Self::muted())
                                .strong(),
                        );
                        ui.add_space(4.0);
                    }

                    if results.is_empty() {
                        ui.label(egui::RichText::new("No snippets found").color(Self::muted()));
                    } else {
                        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                            for (display_index, snippet_index) in results.iter().enumerate() {
                                let snippet = self.snippets[*snippet_index].clone();
                                let selected = display_index == self.quick_selected_index;

                                egui::Frame::none()
                                    .fill(Self::card_bg(selected))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(37, 53, 96),
                                    ))
                                    .inner_margin(egui::Margin::same(10.0))
                                    .show(ui, |ui| {
                                        let title = if snippet.favorite {
                                            format!("★ {}", snippet.title)
                                        } else {
                                            snippet.title.clone()
                                        };

                                        if ui
                                            .selectable_label(
                                                selected,
                                                egui::RichText::new(title).strong(),
                                            )
                                            .clicked()
                                        {
                                            self.quick_selected_index = display_index;
                                            self.activate_quick_search_selection();
                                        }

                                        ui.label(
                                            egui::RichText::new(Self::code_preview(&snippet))
                                                .size(12.0)
                                                .color(Self::muted()),
                                        );
                                    });

                                ui.add_space(4.0);
                            }
                        });
                    }
                });
        }

        if self.new_folder_open {
            egui::Window::new("Create Folder")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Folder name");
                    ui.add_sized(
                        [280.0, 32.0],
                        egui::TextEdit::singleline(&mut self.new_folder_name)
                            .hint_text("Utilities"),
                    );

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if Self::primary_button(ui, "Create").clicked() {
                            self.create_folder();
                        }

                        if Self::action_button(ui, "Cancel").clicked() {
                            self.new_folder_open = false;
                            self.new_folder_name.clear();
                        }
                    });
                });
        }

        if self.template_open {
            egui::Window::new("Template Values")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Fill values for placeholders");
                    ui.add_space(8.0);

                    let fields = self.template_fields.clone();
                    for field in fields {
                        let value = self
                            .template_values
                            .entry(field.clone())
                            .or_insert_with(String::new);

                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [120.0, 28.0],
                                egui::Label::new(
                                    egui::RichText::new(field.clone()).color(Self::muted()),
                                ),
                            );
                            ui.add_sized(
                                [240.0, 28.0],
                                egui::TextEdit::singleline(value).hint_text("value"),
                            );
                        });
                    }

                    ui.add_space(10.0);

                    let preview = render_template(&self.code_input, &self.template_values);
                    egui::CollapsingHeader::new("Preview")
                        .default_open(true)
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(preview)
                                        .monospace()
                                        .size(12.0)
                                        .color(egui::Color32::LIGHT_GRAY),
                                );
                            });
                        });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if Self::primary_button(ui, "Copy Rendered").clicked() {
                            self.confirm_template_copy();
                        }

                        if Self::action_button(ui, "Copy Rendered MD").clicked() {
                            self.template_copy_mode = CopyMode::Markdown;
                            self.confirm_template_copy();
                        }

                        if Self::action_button(ui, "Cancel").clicked() {
                            self.template_open = false;
                            self.template_fields.clear();
                            self.template_values.clear();
                        }
                    });
                });
        }
    }
}

fn extract_placeholders(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;
    let mut set = BTreeSet::new();

    while i + 2 < chars.len() {
        if chars[i] == '$' && chars[i + 1] == '{' {
            let mut j = i + 2;
            let mut buf = String::new();

            while j < chars.len() && chars[j] != '}' {
                buf.push(chars[j]);
                j += 1;
            }

            if j < chars.len() && !buf.trim().is_empty() {
                set.insert(buf.trim().to_string());
                i = j;
            }
        }

        i += 1;
    }

    set.into_iter().collect()
}

fn render_template(text: &str, values: &BTreeMap<String, String>) -> String {
    let mut result = text.to_string();

    for (key, value) in values {
        let pattern = format!("${{{}}}", key);
        result = result.replace(&pattern, value);
    }

    result
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

fn load_app_state() -> AppState {
    if !Path::new(STATE_PATH).exists() {
        return AppState::default();
    }

    let content = match fs::read_to_string(STATE_PATH) {
        Ok(content) => content,
        Err(_) => return AppState::default(),
    };

    if content.trim().is_empty() {
        return AppState::default();
    }

    serde_json::from_str(&content).unwrap_or_default()
}

fn save_app_state(state: &AppState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(STATE_PATH, json).map_err(|e| e.to_string())
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1440.0, 920.0]),
        ..Default::default()
    };

    eframe::run_native(
        "SnippetVault v0.1.0",
        options,
        Box::new(|_cc| Ok(Box::new(SnippetApp::default()))),
    )
}