# SnippetManager 🦀

A modern Rust desktop application for storing, organizing, and reusing code snippets locally.

SnippetManager is a developer-focused snippet manager built entirely in Rust. It is designed to help developers quickly save, edit, search, and reuse useful pieces of code through a fast desktop interface.

The goal of the project is to grow into a polished, Rust-only productivity tool for managing personal code libraries and reusable snippet workflows.

---

## Features (v0.1.0)

- Desktop GUI application
- Create, edit, duplicate, and delete snippets
- Organize snippets by folders
- Pin / favorite important snippets
- Browse snippets in a multi-panel workspace
- Fuzzy search for fast snippet discovery
- Quick search popup for faster navigation
- Syntax-highlighted code editor
- View / edit modes
- Line numbers in the editor
- Copy code to clipboard
- Copy snippets as Markdown code blocks
- Template placeholders like `${name}` for reusable snippets
- Smart rendered copy from templates
- Keyboard shortcuts for common actions
- Import / export snippets in JSON format
- Local JSON storage

Each snippet can include:

- Title
- Description
- Folder
- Programming language
- Tags
- Code content
- Unique ID
- Favorite state
- Created / updated timestamps

All snippet data is stored locally in **`snippets.json`**.  
App state such as folders and recent items is stored in **`snippet_manager_state.json`**.

---

## Running the App

Run the application with:

```bash
cargo run
```
## Tech Stack

Rust

egui / eframe

syntect

fuzzy-matcher

arboard

rfd

Serde

UUID

## Version
### Current version: v0.1.0 ⚙️
#### Adds a more polished desktop experience with folders, favorites, duplicate actions, improved editor workflow, smart copy options, and reusable snippet templates. ⌨️
