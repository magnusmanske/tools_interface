//! # Completer
//! Module for interacting with the [Completer tool](https://completer.toolforge.org/).
//! You can retrieve a list of pages on one wiki taht do not exist in the other.
//! There are blocking and async methods available.
//!
//! ## Example
//! ```rust
//! let mut c = Completer::new("de", "en")
//!     .filter(CompleterFilter::Category{category: "Biologie".to_string(), depth: 0})
//!     .ignore_cache();
//!  c.tool_url = format!("{}/data", mock_server.uri());
//!  c.run().await.unwrap();
//!  c.results()
//!     .iter()
//!     .for_each(|(title, count)| {
//!        println!("{title} wanted {count} times");
//!     });
//! ```

use serde_json::{json, Value};
use crate::ToolsError;

#[derive(Debug, PartialEq)]
/// This is a filter value for `Completer`.
/// It can be a category (with depth), a PetScan ID, or a template.
/// Categories and templates must not have a namespace prefix.
pub enum CompleterFilter {
    Category{category: String, depth: u32},
    PetScan{psid: String},
    Template{template: String},
}

impl CompleterFilter {
    fn to_json(&self) -> Value {
        match self {
            CompleterFilter::Category{category, depth} => {
                json!({
                    "type": "category",
                    "specific": {
                        "title": category,
                        "depth": depth,
                        "talk": false,
                    }
                })
            },
            CompleterFilter::PetScan{psid} => {
                json!({
                    "type": "petscan",
                    "specific": {
                        "id": psid,
                    }
                })
            },
            CompleterFilter::Template{template} => {
                json!({
                    "type": "template",
                    "specific": {
                        "title": template,
                        "talk": false,
                    }
                })
            },
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Completer {
    wiki_from: String,
    wiki_to: String,
    filters: Vec<CompleterFilter>,
    ignore_cache: bool,

    id: u64,
    results: Vec<(String, u64)>,
    tool_url: String,
}

impl Completer {
    /// Finds articles on a wikipedia (`wiki_from`) that are missing on another (`wiki_to`).
    /// **Note**: These are _language codes_ for Wikipedia (eg "de", "en").
    /// This tool olny seems to work on Wikipedia.
    pub fn new(wiki_from: &str, wiki_to: &str) -> Completer {
        Completer {
            wiki_from: wiki_from.to_string(),
            wiki_to: wiki_to.to_string(),
            tool_url: "https://completer.toolforge.org/data".to_string(),
            ..Default::default()
        }
    }

    /// Adds a filter to the completer.
    pub fn filter(mut self, filter: CompleterFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// Tells Completer to ignore the cache.
    pub fn ignore_cache(mut self) -> Self {
        self.ignore_cache = true;
        self
    }

    fn generate_payload(&self) -> Value {
        json!({
            "info": {
                "from": self.wiki_from,
                "to": self.wiki_to,
                "ignoreCache": self.ignore_cache,
                "filters": self.filters.iter().map(|f|f.to_json()).collect::<Vec<Value>>(),
            },
        })
    }

    #[cfg(feature = "tokio")]
    /// Run the query asynchronously.
    pub async fn run(&mut self) -> Result<(), ToolsError> {
        let url = &self.tool_url;
        let j = self.generate_payload();
        // println!("PAYLOAD: {}", j);
        let client = crate::ToolsInterface::tokio_client()?;
        let response = client.post(url).json(&j).send().await?;
        let j: Value = response.json().await?;
        // println!("RESPONSE: {}", j);
        self.from_json(j)
    }

    fn from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        if j["success"].as_bool() != Some(true) {
            return Err(ToolsError::Tool(format!(
                "Completer has failed: {:?}",
                j
            )));
        }
        self.id = j["meta"]["id"].as_u64().ok_or(ToolsError::Tool("No ID".to_string()))?;
        self.results = j["data"]
            .as_array()
            .ok_or(ToolsError::Json("['data'] has no array".into()))?
            .iter()
            .filter_map(|arr| arr.as_array())
            .filter_map(|arr| Some((arr.get(0)?,arr.get(1)?)))
            .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_u64()?)))
            .collect();
        Ok(())
    }

    /// Returns the ID of the query.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the results of the query.
    pub fn results(&self) -> &[(String, u64)] {
        &self.results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, body_json};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn get_mock_server() -> MockServer {
        let mock_path = format!("data");
        let mock_server = MockServer::start().await;
        let bj = json!({"info":{"filters":[{"specific":{"depth":0,"talk":false,"title":"Biologie"},"type":"category"}],"from":"de","ignoreCache":true,"to":"en"}});
        Mock::given(method("POST"))
            .and(body_json(bj))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[["Optimum",4],["Zustandsänderung",1]],"meta":{"cache_age":null,"cached":false,"debugLine":true,"id":6623,"reachedMaxStatementTime":false,"time":"0.08"},"success":true})))
            .mount(&mock_server)
            .await;
        mock_server
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_completer_async() {
        let mock_server = get_mock_server().await;
        let mut c = Completer::new("de", "en")
            .filter(CompleterFilter::Category{category: "Biologie".to_string(), depth: 0})
            .ignore_cache();
        c.tool_url = format!("{}/data", mock_server.uri());
        c.run().await.unwrap();
        assert_eq!(c.id(), 6623);
        assert_eq!(c.results(), &[("Optimum".to_string(), 4), ("Zustandsänderung".to_string(), 1)]);
    }
}
