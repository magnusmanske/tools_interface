use serde_json::Value;
use crate::ToolsError;


#[derive(Debug, Default, PartialEq)]
pub struct PagePile {
    id: u32,
    prefixed_titles: Vec<String>,
    language: Option<String>,
    project: Option<String>,
    wiki: Option<String>,
}

impl PagePile {
    pub fn new(id: u32) -> Self {
        Self { id, ..Default::default() }
    }

    #[cfg(feature = "blocking")]
    pub fn get_blocking(&mut self) -> Result<(), ToolsError> {
        let url = format!("https://pagepile.toolforge.org/api.php?id={id}&action=get_data&doit&format=json", id=self.id);
        let client = crate::ToolsInterface::blocking_client()?;
        let json = client.get(&url).send()?.json()?;
        self.from_json(&json)
    }

    #[cfg(feature = "tokio")]
    pub async fn get(&mut self) -> Result<(), ToolsError> {
        let url = format!("https://pagepile.toolforge.org/api.php?id={id}&action=get_data&doit&format=json", id=self.id);
        let client = crate::ToolsInterface::tokio_client()?;
        let json = client.get(&url).send().await?.json().await?;
        self.from_json(&json)
    }

    fn from_json(&mut self, j: &Value) -> Result<(), ToolsError> {
        self.language = j["language"].as_str().map(|s|s.to_string());
        self.project = j["project"].as_str().map(|s|s.to_string());
        self.wiki = j["wiki"].as_str().map(|s|s.to_string());
        self.prefixed_titles = j["pages"]
            .as_array()
            .ok_or(ToolsError::Json("['pages'] has no rows array".into()))?
            .iter()
            .filter_map(|page| page.as_str())
            .map(|prefixed_title| prefixed_title.to_string())
            .collect();
        let pages_returned = j["pages_returned"].as_i64().ok_or(ToolsError::Json("['pages_returned'] is not an integer".into()))?;
        let pages_total = j["pages_total"].as_i64().ok_or(ToolsError::Json("['pages_total'] is not an integer".into()))?;
        if pages_returned != pages_total {
            return Err(ToolsError::Json(format!("pages_returned ({}) != pages_total ({})", pages_returned, pages_total)));
        }
        if pages_total != self.prefixed_titles.len() as i64 {
            return Err(ToolsError::Json(format!("pages_total ({}) != prefixed_titles.len() ({})", pages_total, self.prefixed_titles.len())));
        }
        Ok(())
    }
    
    pub fn prefixed_titles(&self) -> &[String] {
        &self.prefixed_titles
    }
    
    pub fn language(&self) -> Option<&String> {
        self.language.as_ref()
    }
    
    pub fn project(&self) -> Option<&String> {
        self.project.as_ref()
    }
    
    pub fn wiki(&self) -> Option<&String> {
        self.wiki.as_ref()
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
        pp.get_blocking().unwrap();
        assert_eq!(pp.language().unwrap(),"de");
        assert_eq!(pp.project().unwrap(),"wikipedia");
        assert_eq!(pp.wiki().unwrap(),"dewiki");
        assert_eq!(pp.prefixed_titles().len(), 1747);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pagepile_get_async() {
        let mut pp = PagePile::new(51805);
        pp.get().await.unwrap();
        assert_eq!(pp.language().unwrap(),"de");
        assert_eq!(pp.project().unwrap(),"wikipedia");
        assert_eq!(pp.wiki().unwrap(),"dewiki");
        assert_eq!(pp.prefixed_titles().len(), 1747);
    }
}