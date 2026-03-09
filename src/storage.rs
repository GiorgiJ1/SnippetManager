use crate::models::Snippet;
use std::error::Error;
use std::fs;
use std::path::Path;

const FILE_PATH: &str = "snippets.json";

pub fn load_snippets() -> Result<Vec<Snippet>, Box<dyn Error>> {
    if !Path::new(FILE_PATH).exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(FILE_PATH)?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let snippets: Vec<Snippet> = serde_json::from_str(&content)?;
    Ok(snippets)
}

pub fn save_snippets(snippets: &[Snippet]) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(snippets)?;
    fs::write(FILE_PATH, json)?;
    Ok(())
}