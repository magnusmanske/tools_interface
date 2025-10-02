/// # List building
/// Module for interacting with the [list building](https://list-building.toolforge.org) tool.
/// You can retrieve a list of pages on one wiki that relate to a Wiki page.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let site = Site::from_wiki("enwiki").unwrap();
/// let title = "SARS-CoV-2";
/// let mut a = ListBuilding::new(site, title);
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
pub struct ListBuildingResult {
    pub title: String,
    pub qid: String,
    pub description: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct ListBuilding {
    site: Site,
    title: String,
    results: Vec<ListBuildingResult>,
}

impl ListBuilding {
    pub fn new(site: Site, title: &str) -> Self {
        Self {
            site,
            title: title.to_string(),
            ..Default::default()
        }
    }

    pub fn results(&self) -> &[ListBuildingResult] {
        &self.results
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn title(&self) -> &str {
        &self.title
    }
}

#[async_trait]
impl Tool for ListBuilding {
    #[cfg(feature = "blocking")]
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = format!(
            "https://list-building.toolforge.org/api/serpentine?lang={lang}&title={title}&qid=&k-reader=3&k-links=3&k-morelike=4&wp",
            lang = self.site.language(),
            title = self.title,
        );
        let client = crate::ToolsInterface::blocking_client()?;
        let json = client.get(&url).send()?.json()?;
        self.set_from_json(json)
    }

    #[cfg(feature = "tokio")]
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = format!(
            "https://list-building.toolforge.org/api/serpentine?lang={lang}&title={title}&qid=&k-reader=3&k-links=3&k-morelike=4&wp",
            lang = self.site.language(),
            title = self.title,
        );
        let client = crate::ToolsInterface::tokio_client()?;
        let json = client.get(&url).send().await?.json().await?;
        self.set_from_json(json)
    }

    fn set_from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        for entry in j["results"]
            .as_array()
            .ok_or_else(|| ToolsError::Json("Result is not an array".to_string()))?
        {
            let title = match entry.get("page_title") {
                Some(title) => match title.as_str() {
                    Some(title) => title,
                    None => continue, // Skip row
                },
                None => continue, // Skip row
            };
            let qid = match entry.get("qid") {
                Some(qid) => match qid.as_str() {
                    Some(qid) => qid,
                    None => continue, // Skip row
                },
                None => continue, // Skip row
            };
            let description = match entry.get("description") {
                Some(description) => match description.as_str() {
                    Some(description) => description,
                    None => continue, // Skip row
                },
                None => continue, // Skip row
            };
            self.results.push(ListBuildingResult {
                title: title.to_string(),
                description: description.to_string(),
                qid: qid.to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let site = Site::from_wiki("enwiki").unwrap();
        let title = "SARS-CoV-2";
        let tool = ListBuilding::new(site.clone(), title);
        assert_eq!(tool.site(), &site);
        assert_eq!(tool.title(), title);
    }

    #[tokio::test]
    async fn test_list_building_json() {
        let site = Site::from_wiki("enwiki").unwrap();
        let title = "SARS-CoV-2";
        let mut tool = ListBuilding::new(site, title);
        tool.run().await.unwrap();
        assert!(
            tool.results()
                .iter()
                .any(|result| result.qid == "Q84263196" && result.title == "COVID-19")
        );
    }
}
