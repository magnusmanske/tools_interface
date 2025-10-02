/// # List building
/// Module for interacting with the [list building](https://list-building.toolforge.org) tool.
/// You can retrieve a list of pages on one wiki that relate to a Wiki page.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let site = Site::from_wiki("enwiki").unwrap();
/// let title = "SARS-CoV-2";
/// let mut a = WikiSearch::new(site, title);
/// a.run().await.unwrap();
/// a.results()
///     .iter()
///     .for_each(|result| {
///        println!("Page {} Item {} Description {}", result.title, result.qid, result.description);
///     });
/// ```
use crate::{Site, Tool, ToolsError};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Default, PartialEq)]
pub struct WikiSearchResult {
    pub namespace_id: u32,
    pub title: String,
    pub page_id: usize,
    pub size: usize,
    pub wordcount: usize,
    pub snippet: String,
}

impl WikiSearchResult {
    fn from_json(json: &Value) -> Option<Self> {
        Some(Self {
            namespace_id: json["ns"].as_u64()? as u32,
            title: json["title"].as_str()?.to_string(),
            page_id: json["pageid"].as_u64()? as usize,
            size: json["size"].as_u64()? as usize,
            wordcount: json["wordcount"].as_u64()? as usize,
            snippet: json["snippet"].as_str()?.to_string(),
        })
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct WikiSearch {
    site: Site,
    query: String,
    namespace_ids: String,
    offset: u32,
    limit: u32,
    results: Vec<WikiSearchResult>,
}

impl WikiSearch {
    pub fn new(site: Site, query: &str) -> Self {
        Self {
            site,
            query: query.to_string(),
            namespace_ids: "0".to_string(),
            limit: 10,
            ..Default::default()
        }
    }

    pub fn with_namespace_id(mut self, namespace_id: u32) -> Self {
        self.namespace_ids = format!("{namespace_id}");
        self
    }

    pub fn with_namespace_ids(mut self, namespace_ids: &str) -> Self {
        self.namespace_ids = namespace_ids.to_string();
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = offset;
        self
    }

    pub fn results(&self) -> &[WikiSearchResult] {
        &self.results
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn namespace_ids(&self) -> &str {
        &self.namespace_ids
    }

    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn limit(&self) -> u32 {
        self.limit
    }
}

#[async_trait]
impl Tool for WikiSearch {
    fn get_url(&self) -> String {
        format!(
            "https://{server}/w/api.php?action=query&list=search&srsearch={query}&srnamespace={namespace_id}&sroffset={offset}&srlimit={limit}&format=json",
            server = self.site.webserver(),
            query = self.query,
            namespace_id = self.namespace_ids,
            offset = self.offset,
            limit = self.limit,
        )
    }

    fn set_from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        self.results = j["query"]["search"]
            .as_array()
            .ok_or_else(|| ToolsError::Json("Result is not an array".to_string()))?
            .iter()
            .filter_map(WikiSearchResult::from_json)
            .collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_json() {
        let site = Site::from_wiki("enwiki").unwrap();
        let query = "Heinrich Magnus Manske";
        let mut tool = WikiSearch::new(site, query);
        tool.run().await.unwrap();
        assert!(
            tool.results()
                .iter()
                .any(|result| result.page_id == 3361346 && result.title == "Magnus Manske")
        );
    }
}
