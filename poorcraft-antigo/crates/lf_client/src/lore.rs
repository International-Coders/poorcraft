//! Lore books (build-pack Step 20): readable tomes loaded from
//! `lore/books.toml` at boot. Right-click a tome item to page through it;
//! the Lorekeeper trades them.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct LoreBook {
    pub id: String,
    pub title: String,
    pub item: String,
    pub pages: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct LoreLibrary {
    pub books: Vec<LoreBook>,
}

impl LoreLibrary {
    /// Load from a books.toml file; missing file = empty library (the game
    /// still runs — the chronicle book works regardless).
    pub fn load(path: &std::path::Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn for_item(&self, item_id: &str) -> Option<&LoreBook> {
        self.books.iter().find(|b| b.item == item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Step 20 done-when: book content loads from the real data file and
    /// every tome item maps to readable pages.
    #[test]
    fn lore_books_load_from_the_real_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join("lore/books.toml");
        let lib = LoreLibrary::load(&path);
        assert!(lib.books.len() >= 3, "expected the three tomes, got {}", lib.books.len());
        for book in &lib.books {
            assert!(!book.title.is_empty());
            assert!(book.pages.len() >= 3, "{} too thin", book.id);
            assert!(book.pages.iter().all(|p| p.len() > 40), "{} has empty pages", book.id);
        }
        let forge = lib.for_item("tome_of_the_forge").expect("forge tome maps to its item");
        assert!(forge.pages[0].contains("The Smith"), "anchors the Smith lore thread");
        let null = lib.for_item("tome_of_the_null").expect("null tome maps");
        assert!(null.pages.iter().any(|p| p.contains("Null Knight")), "anchors the Null thread");
        assert!(lib.for_item("not_a_tome").is_none());
    }
}
