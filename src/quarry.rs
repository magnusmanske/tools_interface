use serde_json::Value;

use crate::ToolsError;

#[derive(Debug, Default, PartialEq)]
pub struct Quarry {
    id: u64,
    labels: Vec<String>,
    rows: Vec<Vec<Value>>,
}

impl Quarry {
    pub fn new(id: u64) -> Self {
        Self { id , ..Default::default() }
    }

    #[cfg(feature = "blocking")]
    pub fn get_blocking(&mut self) -> Result<(),ToolsError> {
        let url = format!("https://quarry.wmcloud.org/query/{id}/result/latest/0/json", id=self.id);
        let client = crate::ToolsInterface::blocking_client()?;
        let json: Value = client.get(&url).send()?.json()?;
        self.from_json(&json)
    }

    #[cfg(feature = "tokio")]
    pub async fn get(&mut self) -> Result<(),ToolsError> {
        let url = format!("https://quarry.wmcloud.org/query/{id}/result/latest/0/json", id=self.id);
        let client = crate::ToolsInterface::tokio_client()?;
        let json: Value = client.get(&url).send().await?.json().await?;
        self.from_json(&json)
    }

    fn from_json(&mut self, json: &Value) -> Result<(), ToolsError> {
        self.labels = json.get("headers")
            .ok_or_else(|| ToolsError::Json("No headers in Quarry JSON".to_string()))?
            .as_array()
            .ok_or_else(|| ToolsError::Json("['headers'] is not an array in Quarry JSON".to_string()))?
            .iter()
            .map(|s| s.as_str().unwrap_or("").to_string())
            .collect();

        self.rows = json.get("rows")
            .ok_or_else(|| ToolsError::Json("No rows in Quarry JSON".to_string()))?
            .as_array()
            .ok_or_else(|| ToolsError::Json("Rows is not an array in Quarry JSON".to_string()))?
            .iter()
            .filter_map(|row| row.as_array())
            .map(|row| row.to_vec())
            .collect();

        Ok(())
    }
    
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    pub fn colnum(&self, title: &str) -> Option<usize> {
        self.labels.iter().position(|l| l==title)
    }
    
    pub fn rows(&self) -> &[Vec<Value>] {
        &self.rows
    }
    
    pub fn id(&self) -> u64 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quarry_get_async() {
        let mut quarry = Quarry::new(82868); // dewiki root categories
        quarry.get().await.unwrap();
        let col = quarry.colnum("page_title").unwrap();
        assert_eq!(col, 2);
        assert!(quarry.rows().iter().any(|row| row[col].as_str()==Some("!Hauptkategorie"))
        );
    }

}