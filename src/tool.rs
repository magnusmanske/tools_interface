use async_trait::async_trait;
use serde_json::Value;

use crate::ToolsError;

#[async_trait]
pub trait Tool {

    #[cfg(feature = "blocking")]
    /// Run the tool in a blocking manner.
    fn run_blocking(&mut self) -> Result<(), ToolsError> ;

    #[cfg(feature = "tokio")]
    /// Run the tool asynchronously.
    async fn run(&mut self) -> Result<(), ToolsError> ;

    fn from_json(&mut self, _j: Value) -> Result<(), ToolsError> {
        unimplemented!();
    }

    fn generate_payload(&self) -> Value {
        unimplemented!();
    }

    fn generate_paramters(&self) -> Result<Vec<(String, String)>, ToolsError> {
        unimplemented!();
    }
}