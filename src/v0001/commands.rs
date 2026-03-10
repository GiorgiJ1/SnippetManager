use crate::models::Snippet;
use crate::storage::{load_snippets, save_snippets};
use std::error::Error;
use std::io::{self, Write};

fn read_input(label: &str) -> Result<String, Box<dyn Error>> {
    print!("{}", label);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn read_multiline_code() -> Result<String, Box<dyn Error>> {
    println!("Paste your code below.");
    println!("Type END on a new line when finished:");

    let mut lines = Vec::new();

    loop {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let trimmed = line.trim_end();

        if trimmed == "END" {
            break;
        }

        lines.push(trimmed.to_string());
    }

    Ok(lines.join("\n"))
}

pub fn add_snippet() -> Result<(), Box<dyn Error>> {
    let title = read_input("Title: ")?;
    let language = read_input("Language: ")?;
    let tags_input = read_input("Tags (comma separated): ")?;
    let code = read_multiline_code()?;

    let tags: Vec<String> = tags_input
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();

    let mut snippets = load_snippets()?;
    let snippet = Snippet::new(title, language, tags, code);
    let id = snippet.id;

    snippets.push(snippet);
    save_snippets(&snippets)?;

    println!("Snippet added successfully.");
    println!("ID: {}", id);

    Ok(())
}

pub fn list_snippets() -> Result<(), Box<dyn Error>> {
    let snippets = load_snippets()?;

    if snippets.is_empty() {
        println!("No snippets found.");
        return Ok(());
    }

    for snippet in snippets {
        println!("ID: {}", snippet.id);
        println!("Title: {}", snippet.title);
        println!("Language: {}", snippet.language);
        println!("Tags: {}", snippet.tags.join(", "));
        println!("------------------------------");
    }

    Ok(())
}

pub fn search_snippets(query: &str) -> Result<(), Box<dyn Error>> {
    let snippets = load_snippets()?;
    let q = query.to_lowercase();

    let results: Vec<_> = snippets
        .into_iter()
        .filter(|s| {
            s.title.to_lowercase().contains(&q)
                || s.language.to_lowercase().contains(&q)
                || s.code.to_lowercase().contains(&q)
                || s.tags.iter().any(|tag| tag.to_lowercase().contains(&q))
        })
        .collect();

    if results.is_empty() {
        println!("No matching snippets found.");
        return Ok(());
    }

    for snippet in results {
        println!("ID: {}", snippet.id);
        println!("Title: {}", snippet.title);
        println!("Language: {}", snippet.language);
        println!("Tags: {}", snippet.tags.join(", "));
        println!("------------------------------");
    }

    Ok(())
}

pub fn view_snippet(id: &str) -> Result<(), Box<dyn Error>> {
    let snippets = load_snippets()?;

    let snippet = snippets.iter().find(|s| s.id.to_string() == id);

    match snippet {
        Some(s) => {
            println!("ID: {}", s.id);
            println!("Title: {}", s.title);
            println!("Language: {}", s.language);
            println!("Tags: {}", s.tags.join(", "));
            println!("Code:");
            println!("{}", s.code);
        }
        None => {
            println!("Snippet not found.");
        }
    }

    Ok(())
}

pub fn delete_snippet(id: &str) -> Result<(), Box<dyn Error>> {
    let mut snippets = load_snippets()?;
    let before = snippets.len();

    snippets.retain(|s| s.id.to_string() != id);

    if snippets.len() == before {
        println!("Snippet not found.");
        return Ok(());
    }

    save_snippets(&snippets)?;
    println!("Snippet deleted successfully.");

    Ok(())
}