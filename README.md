# SnippetManager 🦀

A lightweight Rust desktop tool for storing, organizing, and searching code snippets locally.

This project is an early prototype (**v0.0.4**) of a Rust-based snippet manager designed to help developers quickly save, edit, and retrieve useful pieces of code.

The long-term goal is to build a **fast, modern, Rust-only desktop application** for managing personal code libraries.

---

## Features (v0.0.4)

- Desktop GUI application
- Create and edit snippets
- Browse stored snippets
- **Fuzzy search for fast snippet discovery**
- **Syntax-highlighted code editor**
- **Copy code to clipboard**
- **Keyboard shortcuts for common actions**
- **Import / export snippets (JSON)**
- **Tag filtering**
- Delete snippets
- Local JSON storage

Each snippet contains:

- Title  
- Programming language  
- Tags  
- Code content  
- Unique ID  

All data is stored locally in a **`snippets.json`** file.

---

## Running the App

Run the application with:

```bash
cargo run
```
##Tech Stack

Rust

egui / eframe (GUI framework)

syntect (syntax highlighting)

fuzzy-matcher (fuzzy search)

arboard (clipboard support)

rfd (file dialogs)

Serde (JSON serialization)

UUID

## Version

###Current version: v0.0.4 ⚙️

###Adds syntax highlighting, snippet import/export, and tag filtering, making SnippetManager a more complete developer utility. ⌨️
