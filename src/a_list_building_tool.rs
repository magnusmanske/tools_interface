/// # Completer
/// Module for interacting with the [A list building tool](https://a-list-bulding-tool.toolforge.org/).
/// You can retrieve a list of pages on one wiki that relate to a WIkidata item.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let site = Site::from_wiki("enwiki").unwrap();
/// let q = "Q42";
/// let mut a = AListBuildingTool::new(site, q);
/// a.run().await.unwrap();
/// a.results()
///     .iter()
///     .for_each(|result| {
///        println!("Page {} Item {}", result.title, result.qid);
///     });
/// ```
use crate::{Site, Tool, ToolsError};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Default, PartialEq)]
pub struct AListBuildingToolResult {
    pub title: String,
    pub qid: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct AListBuildingTool {
    site: Site,
    q: String,
    results: Vec<AListBuildingToolResult>,
}

impl AListBuildingTool {
    pub fn new(site: Site, q: &str) -> Self {
        Self {
            site,
            q: q.to_string(),
            ..Default::default()
        }
    }

    pub fn results(&self) -> &[AListBuildingToolResult] {
        &self.results
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn q(&self) -> &str {
        &self.q
    }
}

#[async_trait]
impl Tool for AListBuildingTool {
    #[cfg(feature = "blocking")]
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = format!(
            "https://a-list-bulding-tool.toolforge.org/API/?wiki_db={wiki}&QID={q}",
            wiki = self.site.wiki(),
            q = self.q
        );
        let client = crate::ToolsInterface::blocking_client()?;
        let json = client.get(&url).send()?.json()?;
        self.set_from_json(json)
    }

    #[cfg(feature = "tokio")]
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = format!(
            "https://a-list-bulding-tool.toolforge.org/API/?wiki_db={wiki}&QID={q}",
            wiki = self.site.wiki(),
            q = self.q
        );
        let client = crate::ToolsInterface::tokio_client()?;
        let json = client.get(&url).send().await?.json().await?;
        self.set_from_json(json)
    }

    fn set_from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        for entry in j
            .as_array()
            .ok_or_else(|| ToolsError::Json("Result is not an array".to_string()))?
        {
            let title = match entry.get("title") {
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
            self.results.push(AListBuildingToolResult {
                title: title.to_string(),
                qid: qid.to_string(),
            });
        }

        Ok(())
    }
}

// DEACTIVATED WHILE IT ONLY THROWS 500 ERRORS
/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let site = Site::from_wiki("enwiki").unwrap();
        let q = "Q42";
        let tool = AListBuildingTool::new(site.clone(), q);
        assert_eq!(tool.site(), &site);
        assert_eq!(tool.q(), q);
    }

    #[tokio::test]
    async fn test_alistbuildingtool_json() {
        let site = Site::from_wiki("enwiki").unwrap();
        let q = "Q42";
        let mut tool = AListBuildingTool::new(site, q);
        tool.run().await.unwrap();
        assert!(tool
            .results()
            .iter()
            .any(|result| result.qid == "Q5" && result.title == "Human"));
    }
}
*/
