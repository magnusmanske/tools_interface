//! # SparqlRC
//! Module for interacting with the [SparqlRC tool](https://wikidata-todo.toolforge.org/sparql_rc.php).
//! You can retrieve a list of missing topics for a page or category.
//! There are blocking and async methods available.
//!
//! ## Example
//! ```rust
//! let mut rc = SparqlRC::new("SELECT ?q { ?q wdt:P31 wd:Q23413 }")
//!     .start(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap().into())
//!     .end(NaiveDate::from_ymd_opt(2024, 5, 2).unwrap().into());
//! rc.run().await.unwrap();
//! rc.results()
//!     .iter()
//!     .for_each(|entity_edit| {
//!        println!("Entity changed: {}", entity_edit.id);
//!     });
//! ```

use crate::ToolsError;
use chrono::NaiveDateTime;
use serde_json::Value;

#[derive(Debug, Default, PartialEq)]
pub struct EntityEditor {
    pub id: u64,
    pub name: String,
    pub edits: u64,
}

impl EntityEditor {
    fn from_json(j: &Value) -> Option<Self> {
        Some(Self {
            id: j["user_id"].as_str().map(|s| s.parse().ok()).flatten()?,
            name: j["user_text"].as_str()?.to_string(),
            edits: j["edits"].as_u64()?,
        })
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct EntityEdit {
    pub id: String,
    pub label: String,
    pub comment: Option<String>,
    pub msg: Option<String>,
    pub diff_html: Option<String>,
    pub editors: Vec<EntityEditor>,
    pub ts_before: NaiveDateTime,
    pub ts_after: NaiveDateTime,
    pub changed: bool,
    pub created: bool,
    pub reverted: bool,
}

impl EntityEdit {
    fn from_json(j: &Value) -> Option<Self> {
        let ret = Self {
            id: j["id"].as_str().map(|s| s.to_string())?,
            label: j["label"].as_str().map(|s| s.to_string())?,
            comment: j["comment"].as_str().map(|s| s.to_string()),
            msg: j["msg"].as_str().map(|s| s.to_string()),
            diff_html: j["diff"].as_str().map(|s| s.to_string()),
            editors: Self::parse_editors(&j["editors"]),
            ts_before: Self::parse_date(&j["ts_before"])?,
            ts_after: Self::parse_date(&j["ts_after"])?,
            changed: j["changed"].as_bool().unwrap_or(false),
            created: j["created"].as_bool().unwrap_or(false),
            reverted: j["reverted"].as_bool().unwrap_or(false),
        };
        Some(ret)
    }

    fn parse_date(j: &Value) -> Option<NaiveDateTime> {
        let date = j.as_str()?;
        NaiveDateTime::parse_from_str(date, "%Y%m%d%H%M%S").ok()
    }

    fn parse_editors(j: &Value) -> Vec<EntityEditor> {
        j.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|j| EntityEditor::from_json(j))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct SparqlRC {
    sparql: String,
    start: Option<NaiveDateTime>,
    end: Option<NaiveDateTime>,
    languages: Vec<String>,
    no_bot_edits: bool,
    skip_unchanged: bool,

    tool_url: String,
    results: Vec<EntityEdit>,
}

impl SparqlRC {
    /// Create a new SparqlRC object with the given SPARQL query.
    /// The first variable in the SPARQL select statement must be the entity ID, and named "?q".
    pub fn new(sparql: &str) -> Self {
        Self {
            sparql: sparql.into(),
            tool_url: "https://wikidata-todo.toolforge.org/sparql_rc.php".into(),
            ..Default::default()
        }
    }

    /// Set the start date for the query. This is mandatory.
    pub fn start(mut self, start: NaiveDateTime) -> Self {
        self.start = Some(start);
        self
    }

    /// Set the end date for the query.
    pub fn end(mut self, end: NaiveDateTime) -> Self {
        self.end = Some(end);
        self
    }

    fn date2string(dt: &Option<NaiveDateTime>) -> String {
        dt.map(|d| d.format("%Y%m%d%H%M%S").to_string())
            .unwrap_or("".to_string())
    }

    fn generate_paramters(&self) -> Result<Vec<(String, String)>, ToolsError> {
        let parameters: Vec<(String, String)> = [
            ("sparql".into(), self.sparql.clone()),
            ("start".into(), Self::date2string(&self.start)),
            ("end".into(), Self::date2string(&self.end)),
            ("user_lang".into(), self.languages.join(",")),
            ("no_bots".into(), (self.no_bot_edits as u8).to_string()),
            (
                "skip_unchanged".into(),
                (self.skip_unchanged as u8).to_string(),
            ),
            ("format".into(), "json".into()),
        ]
        .into();
        Ok(parameters)
    }

    fn check_start_date(&self) -> Result<(), ToolsError> {
        match self.start {
            Some(_) => Ok(()),
            None => Err(ToolsError::Tool(format!("SparqlRC start date is not set"))),
        }
    }

    #[cfg(feature = "tokio")]
    /// Run the query asynchronously.
    pub async fn run(&mut self) -> Result<(), ToolsError> {
        self.check_start_date()?;
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
        self.check_start_date()?;
        let url = &self.tool_url;
        let parameters = self.generate_paramters()?;
        let client = crate::ToolsInterface::blocking_client()?;
        let j: Value = client.get(url).query(&parameters).send()?.json()?;
        self.from_json(j)
    }

    fn from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        if j["status"].as_str() != Some("OK") {
            return Err(ToolsError::Tool(format!(
                "SparqlRC status is not OK: {:?}",
                j["status"]
            )));
        }
        self.results = j["items"]
            .as_array()
            .ok_or(ToolsError::Json("['items'] has no array".into()))?
            .iter()
            .filter_map(|j| EntityEdit::from_json(j))
            .collect();
        Ok(())
    }

    /// Get the results of the last query.
    pub fn results(&self) -> &[EntityEdit] {
        &self.results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::fs::File;
    use wiremock::matchers::{method, path, query_param_contains};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn get_mock_server() -> MockServer {
        let file = File::open("test_data/sparql_rc.json").expect("file not found");
        let j: Value = serde_json::from_reader(file).expect("error while reading file");
        let mock_path = format!("/sparql_rc.php");
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param_contains(
                "sparql",
                "SELECT ?q { ?q wdt:P31 wd:Q23413 }",
            ))
            .and(query_param_contains("start", "20240501000000"))
            .and(query_param_contains("end", "20240502000000"))
            .and(query_param_contains("no_bots", "0"))
            .and(query_param_contains("skip_unchanged", "0"))
            .and(query_param_contains("format", "json"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(j))
            .mount(&mock_server)
            .await;
        mock_server
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_sparql_rc_async() {
        let mock_server = get_mock_server().await;
        let mut rc = SparqlRC::new("SELECT ?q { ?q wdt:P31 wd:Q23413 }")
            .start(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap().into())
            .end(NaiveDate::from_ymd_opt(2024, 5, 2).unwrap().into());
        rc.tool_url = format!("{}/sparql_rc.php", mock_server.uri());
        rc.run().await.unwrap();
        assert_eq!(rc.results().len(), 26);
        assert_eq!(rc.results()[0].id, "Q121134008");
        assert_eq!(rc.results()[0].label, "Castelluzzo");
        assert_eq!(rc.results()[0].editors.len(), 3);
    }
}

// https://wikidata-todo.toolforge.org/sparql_rc.php?sparql=SELECT+%3Fq+{+%3Fq+wdt%3AP31+wd%3AQ23413+}&start=20240501&end=20240502&user_lang=&sort_mode=last_edit&no_bots=1&skip_unchanged=1&format=json
