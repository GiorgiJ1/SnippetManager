# SnippetManager 🦀

A lightweight Rust desktop tool for storing, organizing, and searching code snippets locally.

This project is an early prototype (**v0.0.3**) of a Rust-based snippet manager designed to help developers quickly save, edit, and retrieve useful pieces of code.

The long-term goal is to build a **fast, modern, Rust-only desktop application** for managing personal code libraries.

---

## Features (v0.0.3)

- Desktop GUI application  
- Create and edit snippets  
- Browse stored snippets  
- **Fuzzy search for faster snippet discovery**  
- **Copy code to clipboard**  
- **Keyboard shortcuts for common actions**  
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

## This launches the Snippet Manager desktop interface where you can create, edit, search, copy, and delete snippets.

Tech Stack

Rust

egui / eframe

Serde (JSON)

UUID

arboard (clipboard support)

fuzzy-matcher (fuzzy search)

Version

### Current version: v0.0.3 ⚙️

### Improved desktop GUI with fuzzy search, clipboard copy functionality, and keyboard shortcuts. ⌨️
