use async_trait::async_trait;
use serde_json::Value;

use crate::ToolsError;

/// Helper function to check if a JSON response has status "OK".
/// Returns an error with the tool name if status is not OK.
pub fn check_ok_status(j: &Value, tool_name: &str) -> Result<(), ToolsError> {
    if j["status"].as_str() != Some("OK") {
        return Err(ToolsError::Tool(format!(
            "{} status is not OK: {:?}",
            tool_name, j["status"]
        )));
    }
    Ok(())
}

/// Helper function to extract a string field from a JSON object.
/// Returns None if the field doesn't exist or isn't a string.
pub fn get_json_string<'a>(entry: &'a Value, field: &str) -> Option<&'a str> {
    entry.get(field)?.as_str()
}

#[async_trait]
pub trait Tool {
    #[cfg(feature = "blocking")]
    /// Run the tool in a blocking manner.
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = self.get_url();
        let client = crate::ToolsInterface::blocking_client()?;
        let json = client.get(&url).send()?.json()?;
        self.set_from_json(json)
    }

    #[cfg(feature = "tokio")]
    /// Run the tool asynchronously.
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = self.get_url();
        let client = crate::ToolsInterface::tokio_client()?;
        let json = client.get(&url).send().await?.json().await?;
        self.set_from_json(json)
    }

    fn set_from_text(&mut self, _text: &str) -> Result<(), ToolsError> {
        unimplemented!();
    }

    fn set_from_json(&mut self, _j: Value) -> Result<(), ToolsError> {
        unimplemented!();
    }

    fn generate_payload(&self) -> Value {
        unimplemented!();
    }

    fn generate_parameters(&self) -> Result<Vec<(String, String)>, ToolsError> {
        unimplemented!();
    }

    fn get_url(&self) -> String {
        unimplemented!();
    }
}
