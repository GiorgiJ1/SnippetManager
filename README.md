# SnippetManager

Snippet Manager (Rust) 🦀

A lightweight developer tool for storing and searching code snippets locally.

This project is an early prototype (v0.0.1) of a Rust-based snippet manager designed to help developers quickly save, organize, and retrieve useful pieces of code.

The goal of the project is to eventually evolve into a fast, modern, Rust-only desktop application for managing personal code libraries.

Features (v0.0.1)

Current prototype includes:

Add new code snippets

List stored snippets

Search snippets by keyword

View snippet details

Delete snippets

Local JSON storage

Snippets contain:

Title

Programming language

Tags

Code content

Unique ID

All data is stored locally in a snippets.json file.

Example Workflow

Add a snippet:

cargo run -- add

List all snippets:

cargo run -- list

Search snippets:

cargo run -- search rust

View snippet:

cargo run -- view SNIPPET_ID

Delete snippet:

cargo run -- delete SNIPPET_ID

Motivation

Developers constantly search for previously written code.
This project explores building a minimal and fast snippet manager using pure Rust.

The initial version focuses on validating the core functionality before introducing a graphical interface.

Roadmap

Future versions may include:

Desktop GUI

Syntax highlighting

Instant search

Tag filtering

Copy-to-clipboard

Import/export snippets

Better snippet organization

Tech Stack

Rust

Serde (JSON serialization)

UUID

Version

Current version: v0.0.1

CLI prototype for testing core functionality.
