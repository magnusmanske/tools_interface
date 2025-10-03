/// # Grep
/// Module for interacting with the [Grep](https://Grep.toolforge.org) tool.
/// You can retrieve a list of pages on one wiki that are nearby a set of coordinates
/// You can query either by coordinates or by a page title.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let site = Site::from_wiki("enwiki").unwrap();
/// let title = "Cambridge";
/// let mut a = Grep::new(site, title);
/// a.run().await.unwrap();
/// a.results()
///     .iter()
///     .for_each(|result| {
///        println!("Page {} Description {} Lat {} Lon {} Image {}", result.title, result.description, result.lat, result.lon, result.image);
///     });
/// ```
use crate::{Site, Tool, ToolsError, fancy_title::FancyTitle};
use async_trait::async_trait;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{Value, json};

#[derive(Debug, Default, PartialEq)]
pub struct Grep {
    site: Site,
    pattern: String,
    namespace_id: usize,
    include_redirects: bool,
    limit_100: bool,
    results: Vec<String>,
}

impl Grep {
    pub fn new(site: Site, query: &str) -> Self {
        Self {
            site,
            pattern: query.to_string(),
            ..Default::default()
        }
    }

    pub fn with_namespace(mut self, namespace_id: usize) -> Self {
        self.namespace_id = namespace_id;
        self
    }

    pub fn limit_100(mut self) -> Self {
        self.limit_100 = true;
        self
    }

    pub fn with_redirects(mut self) -> Self {
        self.include_redirects = true;
        self
    }

    pub fn results(&self) -> &[String] {
        &self.results
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn query(&self) -> &str {
        &self.pattern
    }

    pub async fn as_json(&self) -> Value {
        let namespace_id = self.namespace_id as i64;
        let site = self.site();
        let api = site.api().await.unwrap();
        json!({
            "pages": self.results()
                .iter()
                .map(|result| FancyTitle::new(result, namespace_id, &api).to_json())
                .collect::<Vec<Value>>(),
            "site": site,
        })
    }
}

#[async_trait]
impl Tool for Grep {
    fn get_url(&self) -> String {
        let mut url = format!(
            "https://grep.toolforge.org/index.php?lang={lang}&project={project}&namespace={namespace_id}&pattern={pattern}",
            pattern = self.pattern,
            lang = self.site.language(),
            project = self.site.project(),
            namespace_id = self.namespace_id,
        );
        if self.include_redirects {
            url.push_str("&redirects=on");
        }
        if self.limit_100 {
            url.push_str("&limit=on");
        }
        url
    }

    #[cfg(feature = "blocking")]
    /// Run the tool in a blocking manner.
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = self.get_url();
        let client = crate::ToolsInterface::blocking_client()?;
        let text = client.get(&url).send()?.text()?;
        self.set_from_text(&text)
    }

    #[cfg(feature = "tokio")]
    /// Run the tool asynchronously.
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = self.get_url();
        let client = crate::ToolsInterface::tokio_client()?;
        let text = client.get(&url).send().await?.text().await?;
        self.set_from_text(&text)
    }

    fn set_from_text(&mut self, text: &str) -> Result<(), ToolsError> {
        lazy_static! {
            static ref RE_PAGE: Regex = Regex::new(r#"<li><a href=".*?">(.+?)</a></li>"#)
                .expect("Regex pattern should be valid");
        }
        self.results = RE_PAGE
            .captures_iter(text)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let site = Site::from_wiki("enwiki").unwrap();
        let query = "^Mag.*ske$";
        let tool = Grep::new(site.clone(), query);
        assert_eq!(tool.site(), &site);
        assert_eq!(tool.query(), query);
    }

    #[tokio::test]
    async fn test_json() {
        let site = Site::from_wiki("enwiki").unwrap();
        let query = "^Mag.*ske$";
        let mut tool = Grep::new(site.clone(), query);
        tool.run().await.unwrap();
        assert!(
            tool.results()
                .iter()
                .any(|result| result == "Magnus Manske")
        );
    }
}
