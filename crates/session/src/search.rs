use anyhow::Result;

pub struct SessionSearch;

impl SessionSearch {
    pub fn new() -> Self {
        Self
    }

    pub fn search(&self, query: &str, _limit: usize) -> Result<Vec<SearchResult>> {
        println!("Searching sessions for: {}", query);
        Ok(vec![])
    }
}

pub struct SearchResult {
    pub session_id: String,
    pub snippet: String,
}

impl Default for SessionSearch {
    fn default() -> Self {
        Self::new()
    }
}
