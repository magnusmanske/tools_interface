//! # QuickStatements
//! This module provides a way to run QuickStatements commands server-side.
//! You can add commands and run them in a batch.
//! This requires your username and a QuickStatements token.
//! The token can be obtained from https://tools.wmflabs.org/quickstatements/#/user when logged in.
//! For this to work, you need to have run a batch (server side) before manually
//! (that is, in the QuickStatements web interface), so your OAuth details can be filled in once.
//! There are blocking and async methods available.
//!
//! ## Example
//! ```rust
//! let mut qs = QuickStatements::new("Your user name", "Your PetScan token").batch_name("My batch");
//! qs.add_command("Q4115189\tP31\tQ1");
//! qs.run().await.unwrap();
//! let batch_id = qs.batch_id().unwrap();
//! ```

use crate::{Tool, ToolsError};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Default, PartialEq)]
pub struct QuickStatements {
    petscan_uri: String, // For testing
    token: String,
    user_name: String,
    compress: bool,
    batch_name: String,
    site: String,
    commands: String, // V1

    batch_id: Option<u64>,
}

impl QuickStatements {
    /// Create a new QuickStatements object.
    /// Requires your username and a QuickStatements token.
    /// The token can be obtained from https://tools.wmflabs.org/quickstatements/#/user when logged in.
    /// For this to work, you need to have run a batch (server side) before manually, so your OAuth details can be filled in once.
    pub fn new<S1: Into<String>, S2: Into<String>>(user_name: S1, token: S2) -> Self {
        Self {
            petscan_uri: "https://quickstatements.toolforge.org/api.php".to_string(),
            token: token.into(),
            user_name: user_name.into(),
            compress: true,
            site: "wikidata".to_string(),
            ..Default::default()
        }
    }

    /// Give the batch a name. This is optional.
    pub fn batch_name<S: Into<String>>(mut self, batch_name: S) -> Self {
        self.batch_name = batch_name.into();
        self
    }

    /// Deactivate compression.
    /// In case there is a problem with complex CREATE commands.
    pub fn no_compression(mut self) -> Self {
        self.compress = false;
        self
    }

    /// Adds a tab-separated (V1) QS command.
    pub fn add_command(&mut self, command: &str) {
        self.commands += &format!("{}\n", command);
    }

    pub fn batch_id(&self) -> Option<u64> {
        self.batch_id
    }
}

#[async_trait]
impl Tool for QuickStatements {
    fn generate_paramters(&self) -> Result<Vec<(String, String)>, ToolsError> {
        let params = [
            ("action", "import"),
            ("submit", "1"),
            ("format", "v1"),
            ("token", &self.token),
            ("username", &self.user_name),
            ("batchname", &self.batch_name),
            ("data", &self.commands),
            ("compress", if self.compress { "1" } else { "0" }),
            ("site", &self.site),
        ];
        let ret = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Ok(ret)
    }

    #[cfg(feature = "blocking")]
    /// Starts the server-side batch and consumes the QuickStatements object.
    /// Returns the batch ID if successful.
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = &self.petscan_uri;
        let params = self.generate_paramters()?;
        let client = crate::ToolsInterface::blocking_client()?;
        let j: Value = client.post(url).form(&params).send()?.json()?;
        let status = j["status"]
            .as_str()
            .ok_or(ToolsError::Json("['status'] is not a string".into()))?;
        if status != "OK" {
            return Err(ToolsError::Json(format!(
                "QuickStatements status is not OK: {:?}",
                status
            )));
        }
        self.batch_id = j["batch_id"].as_u64();
        Ok(())
    }

    #[cfg(feature = "tokio")]
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = &self.petscan_uri;
        let params = self.generate_paramters()?;
        let client = crate::ToolsInterface::tokio_client()?;
        let response = client.post(url).form(&params).send().await?;
        let j: Value = response.json().await?;

        let status = j["status"]
            .as_str()
            .ok_or(ToolsError::Json("['status'] is not a string".into()))?;
        if status != "OK" {
            return Err(ToolsError::Json(format!(
                "QuickStatements status is not OK: {:?}",
                status
            )));
        }
        self.batch_id = j["batch_id"].as_u64();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_quickstatements_run_async() {
        let mock_path = format!("/api.php");
        let mock_server = MockServer::start().await;
        let token = "FAKE_TOKEN";
        Mock::given(method("POST"))
            .and(body_string_contains("action=import"))
            .and(body_string_contains("submit=1"))
            .and(body_string_contains("format=v1"))
            .and(body_string_contains("username=Magnus_Manske"))
            .and(body_string_contains(token))
            .and(body_string_contains("batchname=foobar"))
            .and(body_string_contains("compress=1"))
            .and(body_string_contains("Q4115189%09P31%09Q1"))
            .and(body_string_contains("site=wikidata"))
            .and(path(&mock_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "batch_id": 12345,
                "debug": {
                    "format": "v1",
                    "openpage": 0,
                    "temporary": false
                },
                "site": "wikidata",
                "status": "OK"
            })))
            .mount(&mock_server)
            .await;
        let mut qs = QuickStatements::new("Magnus_Manske", token).batch_name("foobar");
        qs.petscan_uri = format!("{}{mock_path}", mock_server.uri());
        qs.add_command("Q4115189\tP31\tQ1");
        qs.run().await.unwrap();
        assert_eq!(qs.batch_id(), Some(12345));
    }
}
