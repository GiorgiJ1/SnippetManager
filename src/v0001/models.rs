use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Snippet {
    pub id: Uuid,
    pub title: String,
    pub language: String,
    pub tags: Vec<String>,
    pub code: String,
}

impl Snippet {
    pub fn new(title: String, language: String, tags: Vec<String>, code: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            language,
            tags,
            code,
        }
    }
}