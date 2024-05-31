//! # Missing Topics
//! Module for interacting with the [Missing Topics tool](https://missingtopics.toolforge.org/).
//! You can retrieve a list of missing topics for a page or category.
//! There are blocking and async methods available.
//!
//! ## Example
//! ```rust
//! let mut mt = MissingTopics::new(Site::from_wiki("dewiki").unwrap())
//!     .with_article("Biologie")
//!     .no_template_links(true);
//! mt.run().await.unwrap();
//! mt.results()
//!     .iter()
//!     .for_each(|(title, count)| {
//!        println!("{title} wanted {count} times");
//!     });
//! ```

use crate::{Site, ToolsError};
use serde_json::Value;

#[derive(Debug, Default, PartialEq)]
pub struct MissingTopics {
    site: Site,
    category_depth: Option<u32>,
    category: Option<String>,
    article: Option<String>,
    occurs_more_often_than: Option<u32>,
    no_template_links: Option<bool>,
    no_singles: bool,

    url_used: String,
    results: Vec<(String, u64)>,
    tool_url: String,
}

impl MissingTopics {
    /// Create a new MissingTopics object with the given site.
    pub fn new(site: Site) -> Self {
        Self {
            site,
            tool_url: "https://missingtopics.toolforge.org/".to_string(),
            ..Default::default()
        }
    }

    /// Set the category and category depth for the query.
    /// The category depth is the number of subcategories to include.
    /// Only one of category or article can be set.
    pub fn with_category(mut self, category: &str, category_depth: u32) -> Self {
        self.category = Some(category.into());
        self.category_depth = Some(category_depth);
        self
    }

    /// Set the article for the query.
    /// Only one of category or article can be set.
    pub fn with_article(mut self, article: &str) -> Self {
        self.article = Some(article.into());
        self
    }

    /// Any result must have more than the given number of occurrences.
    pub fn limit(mut self, occurs_more_often_than: u32) -> Self {
        self.no_singles = true;
        self.occurs_more_often_than = Some(occurs_more_often_than);
        self
    }

    /// Filter out links from templates used in category pages.
    pub fn no_template_links(mut self, no_template_links: bool) -> Self {
        self.no_template_links = Some(no_template_links);
        self
    }

    fn generate_paramters(&self) -> Result<Vec<(String, String)>, ToolsError> {
        let mut parameters: Vec<(String, String)> = [
            ("language".to_string(), self.site.language().to_string()),
            ("project".to_string(), self.site.project().to_string()),
            ("doit".to_string(), "Run".to_string()),
            ("wikimode".to_string(), "json".to_string()),
        ]
        .to_vec();

        if self.category.is_some() && self.category_depth.is_some() && self.article.is_some() {
            return Err(ToolsError::Tool(
                "Only one of category or article can be set".to_string(),
            ));
        }
        if let (Some(category), Some(category_depth)) = (&self.category, &self.category_depth) {
            parameters.push(("category".to_string(), category.to_string()));
            parameters.push(("depth".to_string(), category_depth.to_string()));
        } else if let Some(article) = &self.article {
            parameters.push(("article".to_string(), article.to_string()));
        } else {
            return Err(ToolsError::Tool(
                "Either category or article must be set".to_string(),
            ));
        }
        match self.no_singles {
            true => parameters.push(("nosingles".to_string(), "1".to_string())),
            false => parameters.push(("nosingles".to_string(), "0".to_string())),
        }
        match self.no_template_links {
            Some(true) => parameters.push(("no_template_links".to_string(), "1".to_string())),
            Some(false) => parameters.push(("no_template_links".to_string(), "0".to_string())),
            _ => {}
        }
        if let Some(occurs_more_often_than) = self.occurs_more_often_than {
            parameters.push(("limitnum".to_string(), occurs_more_often_than.to_string()));
        }
        Ok(parameters)
    }

    #[cfg(feature = "tokio")]
    /// Run the query asynchronously.
    pub async fn run(&mut self) -> Result<(), ToolsError> {
        let url = &self.tool_url;
        let parameters = self.generate_paramters()?;
        let client = crate::ToolsInterface::tokio_client()?;
        let response = client.get(url).query(&parameters).send().await?;
        let j: Value = response.json().await?;
        self.from_json(j)
    }

    #[cfg(feature = "blocking")]
    /// Run the query in a blocking manner.
    pub fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = &self.tool_url;
        let parameters = self.generate_paramters()?;
        let client = crate::ToolsInterface::blocking_client()?;
        let j: Value = client.get(url).query(&parameters).send()?.json()?;
        self.from_json(j)
    }

    fn from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        if j["status"].as_str() != Some("OK") {
            return Err(ToolsError::Tool(format!(
                "MissingTopics status is not OK: {:?}",
                j["status"]
            )));
        }
        self.results = j["results"]
            .as_object()
            .ok_or(ToolsError::Json("['results'] has no object".into()))?
            .iter()
            .filter_map(|(k, v)| Some((k.to_string(), v.as_u64()?)))
            .collect();
        self.url_used = j["url"]
            .as_str()
            .ok_or(ToolsError::Json("['url'] is missing".into()))?
            .to_string();
        Ok(())
    }

    /// Get the URL used for the last query.
    pub fn url_used(&self) -> &str {
        &self.url_used
    }

    /// Get the results of the last query.
    /// The results are a list of tuples with the missing article and the number of occurrences.
    pub fn results(&self) -> &[(String, u64)] {
        &self.results
    }

    /// Get the site used for the query.
    pub fn site(&self) -> &Site {
        &self.site
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Site;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param_contains};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn get_mock_server() -> MockServer {
        let mock_path = format!("/");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param_contains("language","de"))
            .and(query_param_contains("project","wikipedia"))
            .and(query_param_contains("article","Biologie"))
            .and(query_param_contains("language","de"))
            .and(query_param_contains("doit","Run"))
            .and(query_param_contains("wikimode","json"))
            .and(query_param_contains("no_template_links","1"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results":{"Ethnobiologie":4,"Landschaftsdiversität":3,"Micrographia":4,"Spezielle_Botanik":6,"Wbetavirus":4,"Zellphysiologie":4},"status":"OK","url":"https://missingtopics.toolforge.org/?language=de&project=wikipedia&depth=1&category=&article=Biologie&wikimode=json&limitnum=1&notemplatelinks=0"})))
            .mount(&mock_server)
            .await;
        mock_server
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_missing_topics_run_async() {
        let mock_server = get_mock_server().await;
        let mut mt = MissingTopics::new(Site::from_wiki("dewiki").unwrap())
            .with_article("Biologie")
            .no_template_links(true);
        mt.tool_url = format!("{}/", mock_server.uri());
        mt.run().await.unwrap();
        assert_eq!(mt.results.len(), 6);
        assert_eq!(mt.results[5].0, "Zellphysiologie");
        assert_eq!(mt.results[5].1, 4);
        assert_eq!(mt.url_used, "https://missingtopics.toolforge.org/?language=de&project=wikipedia&depth=1&category=&article=Biologie&wikimode=json&limitnum=1&notemplatelinks=0")
    }
}
