# SnippetManager 🦀

A lightweight Rust desktop tool for storing, organizing, and searching code snippets locally.

This project is an early prototype (**v0.0.2**) of a Rust-based snippet manager designed to help developers quickly save and retrieve useful pieces of code.

The long-term goal is to build a **fast, modern, Rust-only desktop application** for managing personal code libraries.

---

## Features (v0.0.2)

- Desktop GUI application  
- Create and edit snippets  
- Browse stored snippets  
- Search snippets by keyword  
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

This launches the Snippet Manager desktop interface where you can create, edit, search, and delete snippets.

Tech Stack

Rust

egui / eframe

Serde (JSON)

UUID

Version

Current version: v0.0.2

First desktop GUI prototype of Snippet Manager.
