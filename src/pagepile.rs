/// # PagePile
/// Module for interacting with the PagePile tool.
/// You can retrieve the list of pages in a PagePile by ID.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let mut pp = PagePile::new(12345); // Your PagePile ID
/// pp.get().await.unwrap();
/// let wiki = pp.wiki().unwrap();
/// let page_titles = pp.prefixed_titles();
/// ```
use crate::{Site, Tool, ToolsError};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Default, PartialEq)]
pub struct PagePile {
    id: u32,

    prefixed_titles: Vec<String>,
    language: Option<String>,
    project: Option<String>,
    wiki: Option<String>,
}

impl PagePile {
    /// Creates a new PagePile with the given ID.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Returns the namespace-prefixed pages in the PagePile.
    pub fn prefixed_titles(&self) -> &[String] {
        &self.prefixed_titles
    }

    /// Returns the language for the PagePile, if known.
    pub fn language(&self) -> Option<&String> {
        self.language.as_ref()
    }

    /// Returns the project for the PagePile, if known.
    pub fn project(&self) -> Option<&String> {
        self.project.as_ref()
    }

    /// Returns the wiki for the PagePile, if known.
    pub fn wiki(&self) -> Option<&String> {
        self.wiki.as_ref()
    }

    /// Returns the site for the PagePile, if wither the wiki, or the language and project are known.
    pub fn site(&self) -> Option<crate::Site> {
        Some(match &self.wiki {
            Some(wiki) => Site::from_wiki(wiki)?,
            None => Site::from_language_project(self.language.as_ref()?, self.project.as_ref()?),
        })
    }
}

#[async_trait]
impl Tool for PagePile {
    fn get_url(&self) -> String {
        format!(
            "https://pagepile.toolforge.org/api.php?id={id}&action=get_data&doit&format=json",
            id = self.id
        )
    }

    fn set_from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        self.language = j["language"].as_str().map(|s| s.to_string());
        self.project = j["project"].as_str().map(|s| s.to_string());
        self.wiki = j["wiki"].as_str().map(|s| s.to_string());
        self.prefixed_titles = j["pages"]
            .as_array()
            .ok_or(ToolsError::Json("['pages'] has no rows array".into()))?
            .iter()
            .filter_map(|page| page.as_str())
            .map(|prefixed_title| prefixed_title.to_string())
            .collect();
        let pages_returned = j["pages_returned"].as_i64().ok_or(ToolsError::Json(
            "['pages_returned'] is not an integer".into(),
        ))?;
        let pages_total = j["pages_total"]
            .as_i64()
            .ok_or(ToolsError::Json("['pages_total'] is not an integer".into()))?;
        if pages_returned != pages_total {
            return Err(ToolsError::Json(format!(
                "pages_returned ({}) != pages_total ({})",
                pages_returned, pages_total
            )));
        }
        if pages_total != self.prefixed_titles.len() as i64 {
            return Err(ToolsError::Json(format!(
                "pages_total ({}) != prefixed_titles.len() ({})",
                pages_total,
                self.prefixed_titles.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagepile_new() {
        let pp = PagePile::new(1);
        assert_eq!(pp.id, 1);
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn test_pagepile_get_blocking() {
        let mut pp = PagePile::new(51805);
        pp.run_blocking().unwrap();
        assert_eq!(pp.language().unwrap(), "de");
        assert_eq!(pp.project().unwrap(), "wikipedia");
        assert_eq!(pp.wiki().unwrap(), "dewiki");
        assert_eq!(pp.prefixed_titles().len(), 1747);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pagepile_get_async() {
        let mut pp = PagePile::new(51805);
        pp.run().await.unwrap();
        assert_eq!(pp.language().unwrap(), "de");
        assert_eq!(pp.project().unwrap(), "wikipedia");
        assert_eq!(pp.wiki().unwrap(), "dewiki");
        assert_eq!(pp.prefixed_titles().len(), 1747);
    }
}
