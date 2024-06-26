/// # Quarry
/// A module for interacting with the Quarry web service.
/// You can query for the latest results of a Quarry query via the query ID.
/// There are blocking and async methods available.
///
/// ## Example
/// ```rust
/// #[tokio::main]
/// async fn main() {
///     let mut quarry = Quarry::new(12345); // Your Quarry query ID
///     quarry.get().await.unwrap();
///     let column_number = quarry.colnum("page_title").unwrap();
///     let page_titles = quarry.rows().iter()
///         .filter_map(|row| row[column_number].as_str())
///         .collect::<Vec<_>>();
/// }
/// ```
use async_trait::async_trait;
use serde_json::Value;

use crate::{Tool, ToolsError};

#[derive(Debug, Default, PartialEq)]
pub struct Quarry {
    id: u64,
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
}

impl Quarry {
    /// Initialize with a valid Quarry ID.
    pub fn new(id: u64) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    /// Get the column titles.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// Get the column number for a given title.
    pub fn colnum(&self, title: &str) -> Option<usize> {
        self.columns.iter().position(|l| l == title)
    }

    /// Get the rows.
    pub fn rows(&self) -> &[Vec<Value>] {
        &self.rows
    }

    /// Get the Quarry ID.
    pub fn id(&self) -> u64 {
        self.id
    }
}

#[async_trait]
impl Tool for Quarry {
    #[cfg(feature = "blocking")]
    /// Download the latest results from Quarry.
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = format!(
            "https://quarry.wmcloud.org/query/{id}/result/latest/0/json",
            id = self.id
        );
        let client = crate::ToolsInterface::blocking_client()?;
        let json: Value = client.get(&url).send()?.json()?;
        self.from_json(json)
    }

    #[cfg(feature = "tokio")]
    /// Download the latest results from Quarry.
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = format!(
            "https://quarry.wmcloud.org/query/{id}/result/latest/0/json",
            id = self.id
        );
        let client = crate::ToolsInterface::tokio_client()?;
        let json: Value = client.get(&url).send().await?.json().await?;
        self.from_json(json)
    }

    fn from_json(&mut self, json: Value) -> Result<(), ToolsError> {
        self.columns = json
            .get("headers")
            .ok_or_else(|| ToolsError::Json("No headers in Quarry JSON".to_string()))?
            .as_array()
            .ok_or_else(|| {
                ToolsError::Json("['headers'] is not an array in Quarry JSON".to_string())
            })?
            .iter()
            .map(|s| s.as_str().unwrap_or("").to_string())
            .collect();

        self.rows = json
            .get("rows")
            .ok_or_else(|| ToolsError::Json("No rows in Quarry JSON".to_string()))?
            .as_array()
            .ok_or_else(|| ToolsError::Json("Rows is not an array in Quarry JSON".to_string()))?
            .iter()
            .filter_map(|row| row.as_array())
            .map(|row| row.to_vec())
            .collect();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "blocking")]
    #[test]
    fn test_quarry_get_blocking() {
        let mut quarry = Quarry::new(82868); // dewiki root categories
        quarry.run_blocking().unwrap();
        let column_number = quarry.colnum("page_title").unwrap();
        assert_eq!(column_number, 2);
        assert!(quarry
            .rows()
            .iter()
            .any(|row| row[column_number].as_str() == Some("!Hauptkategorie")));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_quarry_get_async() {
        let mut quarry = Quarry::new(82868); // dewiki root categories
        quarry.run().await.unwrap();
        let column_number = quarry.colnum("page_title").unwrap();
        assert_eq!(column_number, 2);
        assert!(quarry
            .rows()
            .iter()
            .any(|row| row[column_number].as_str() == Some("!Hauptkategorie")));
    }
}
