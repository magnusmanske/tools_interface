/// # Duplicity
/// Module for interacting with the [Duplicity tool](https://wikidata-todo.toolforge.org/duplicity/).
/// You can retrieve a list of pages on a wiki that do not have a Wikidata item.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let wikis = Duplicity::wikis().await.unwrap();
/// wikis
///     .iter()
///     .for_each(|(wiki, count)| println!("{wiki} has {count} pages without WIkidata item"));
///
/// let mut d = Duplicity::new(Site::from_wiki("enwiki").unwrap());
/// d.run().await.unwrap();
/// d.results()
///     .iter()
///     .for_each(|result| {
///        println!("{} was added {}",result.title, result.creation_date);
///     });
/// ```
use crate::{Site, Tool, ToolsError};
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::Value;

#[derive(Debug, Default, PartialEq)]
pub struct DuplicityResult {
    pub title: String,
    pub creation_date: NaiveDateTime,
}

impl DuplicityResult {
    fn from_json(j: &Value) -> Result<Self, ToolsError> {
        let title = j["title"]
            .as_str()
            .ok_or_else(|| ToolsError::Json("DuplicityResult title is not a string".to_string()))?
            .to_string();

        let creation_date = j["creation_date"].as_str().ok_or_else(|| {
            ToolsError::Json("DuplicityResult creation_date is not a string".to_string())
        })?;
        let creation_date =
            NaiveDateTime::parse_from_str(creation_date, "%Y%m%d%H%M%S").map_err(|e| {
                ToolsError::Json(format!("DuplicityResult creation_date parse error: {}", e))
            })?;
        Ok(Self {
            title,
            creation_date,
        })
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Duplicity {
    site: Site,
    results: Vec<DuplicityResult>,
}

impl Duplicity {
    pub async fn wikis() -> Result<Vec<(String, u64)>, ToolsError> {
        let url = "https://wikidata-todo.toolforge.org/duplicity/api.php?action=wikis";
        let client = crate::ToolsInterface::tokio_client()?;
        let response = client.get(url).send().await?;
        let j: Value = response.json().await?;
        let ret = j["wikis"]
            .as_array()
            .ok_or_else(|| ToolsError::Json("['wikis'] is not an array".to_string()))?
            .iter()
            .filter_map(|x| {
                let wiki = x.get("wiki")?.as_str()?.to_string();
                let cnt = x.get("cnt")?.as_str()?.parse::<u64>().ok()?;
                Some((wiki, cnt))
            })
            .collect();
        Ok(ret)
    }

    pub fn new(site: Site) -> Self {
        Self {
            site,
            ..Default::default()
        }
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn results(&self) -> &[DuplicityResult] {
        &self.results
    }
}

#[async_trait]
impl Tool for Duplicity {
    fn generate_paramters(&self) -> Result<Vec<(String, String)>, ToolsError> {
        let parameters: Vec<(String, String)> = [
            ("action".to_string(), "articles".to_string()),
            ("wiki".to_string(), self.site.wiki().to_string()),
        ]
        .to_vec();
        Ok(parameters)
    }

    #[cfg(feature = "blocking")]
    /// Run the query in a blocking manner.
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = "https://wikidata-todo.toolforge.org/duplicity/api.php";
        let parameters = self.generate_paramters()?;
        let client = crate::ToolsInterface::blocking_client()?;
        let j: Value = client.get(url).query(&parameters).send()?.json()?;
        self.set_from_json(j)
    }

    #[cfg(feature = "tokio")]
    /// Run the query asynchronously.
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = "https://wikidata-todo.toolforge.org/duplicity/api.php";
        let parameters = self.generate_paramters()?;
        let client = crate::ToolsInterface::tokio_client()?;
        let response = client.get(url).query(&parameters).send().await?;
        let j: Value = response.json().await?;
        self.set_from_json(j)
    }

    fn set_from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        if j["status"].as_str() != Some("OK") {
            return Err(ToolsError::Tool(format!(
                "MissingTopics status is not OK: {:?}",
                j["status"]
            )));
        }
        self.results = j["articles"]
            .as_array()
            .ok_or_else(|| ToolsError::Json("['results'] is not an array".to_string()))?
            .iter()
            .map(DuplicityResult::from_json)
            .collect::<Result<Vec<DuplicityResult>, ToolsError>>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_duplicity_wikis_async() {
        let wikis = Duplicity::wikis().await.unwrap();
        assert!(wikis.len() > 300);
        assert!(wikis.iter().any(|(wiki, _count)| wiki == "kswiki"));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_duplicity_run_async() {
        let mut d = Duplicity::new(Site::from_wiki("enwiki").unwrap());
        d.run().await.unwrap();
        assert_eq!(d.site().language(), "en");
        assert_eq!(d.site().project(), "wikipedia");
        assert!(d.results().len() > 1000); // enwiki is usually bad.
    }
}
