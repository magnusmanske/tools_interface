/// # PetScan
/// This module provides a wrapper around the PetScan tool.
/// You can perform a PetScan query via a PSID.
/// There are blocking and async methods available.
///
/// ## Example
/// ```rust
/// let mut ps = PetScan::new(12345); // Your PSID
/// ps.parameters_mut().push(("foo".to_string(), "bar".to_string())); // Override parameters from the PSID
/// ps.get().await.unwrap();
/// let page_titles = ps.pages.iter().map(|page| page.page_title).collect::<Vec<_>>();
/// ```
use std::collections::HashMap;

use crate::{Tool, ToolsError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PetScanFileUsage {
    pub ns: i32,
    pub page: String,
    pub wiki: String,
}

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PetScanMetadata {
    // TODO defaultsort (fix JSON output upstream)
    #[serde(default)]
    pub coordinates: String, // Coordinates "lat/lon"
    #[serde(default)]
    pub image: String, // Page image
    #[serde(default)]
    pub wikidata: String, // Wikidata item
    #[serde(default)]
    pub disambiguation: bool, // Is disambiguation page
    #[serde(default)]
    pub fileusage: String,
    #[serde(default)]
    pub img_height: u64,
    #[serde(default)]
    pub img_width: u64,
    #[serde(default)]
    pub img_major_mime: String,
    #[serde(default)]
    pub img_media_type: String,
    #[serde(default)]
    pub img_minor_mime: String,
    #[serde(default)]
    pub img_sha1: String,
    #[serde(default)]
    pub img_size: u64,
    #[serde(default)]
    pub img_timestamp: String,
    #[serde(default)]
    pub img_user_text: String,
}

impl PetScanMetadata {
    pub fn coordinates(&self) -> Option<(f64, f64)> {
        let mut parts = self.coordinates.split('/');
        let lat = parts.next()?.parse().ok()?;
        let lon = parts.next()?.parse().ok()?;
        Some((lat, lon))
    }
}

#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct PetScanPage {
    pub page_id: u32,
    pub page_latest: String,
    pub page_len: u32,
    pub page_namespace: i64,
    pub page_title: String,
    #[serde(default)]
    pub giu: Vec<PetScanFileUsage>,
    #[serde(default)]
    pub metadata: PetScanMetadata,
}

impl Into<mediawiki::title::Title> for PetScanPage {
    fn into(self) -> mediawiki::title::Title {
        let title_with_spaces = mediawiki::title::Title::underscores_to_spaces(&self.page_title);
        mediawiki::title::Title::new(&title_with_spaces, self.page_namespace)
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct PetScan {
    psid: u32,
    parameters: Vec<(String, String)>,
    pages: Vec<PetScanPage>,
    namespaces: HashMap<i32, String>,
    query: Option<String>,
    wiki: Option<String>,
    status: Option<String>,
}

impl PetScan {
    /// Create a new PetScan query with a PSID.
    pub fn new(psid: u32) -> Self {
        Self {
            psid,
            ..Default::default()
        }
    }

    /// Get the mutable parameters for the future PetScan query.
    /// You can override the parameters from the PSID this way.
    pub fn parameters_mut(&mut self) -> &mut Vec<(String, String)> {
        &mut self.parameters
    }

    /// Get the namespaces from the PetScan query.
    pub fn pages(&self) -> &[PetScanPage] {
        &self.pages
    }

    /// Get the (main) wiki from the PetScan query.
    pub fn wiki(&self) -> Option<&String> {
        self.wiki.as_ref()
    }

    /// Get the PetScan query that was run.
    pub fn query(&self) -> Option<&String> {
        self.query.as_ref()
    }
}

#[async_trait]
impl Tool for PetScan {
    #[cfg(feature = "blocking")]
    /// Perform a blocking PetScan query.
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = format!("https://petscan.wmflabs.org/?psid={psid}&format=json&output_compatability=quick-intersection", psid=self.psid);
        let client = crate::ToolsInterface::blocking_client()?;
        let j: Value = client.get(&url).query(&self.parameters).send()?.json()?;
        self.from_json(j)
    }

    #[cfg(feature = "tokio")]
    /// Get the PetScan query asynchronously.
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = format!("https://petscan.wmflabs.org/?psid={psid}&format=json&output_compatability=quick-intersection", psid=self.psid);
        let client = crate::ToolsInterface::tokio_client()?;
        let j = client
            .get(&url)
            .query(&self.parameters)
            .send()
            .await?
            .json()
            .await?;
        self.from_json(j)
    }

    fn from_json(&mut self, json: Value) -> Result<(), ToolsError> {
        self.status = json["status"].as_str().map(|s| s.to_string());
        if self.status != Some("OK".to_string()) {
            return Err(ToolsError::Tool(format!(
                "PetScan status is not OK: {:?}",
                self.status
            )));
        }
        self.query = json["query"].as_str().map(|s| s.to_string());
        self.namespaces = json["namespaces"]
            .as_object()
            .ok_or(ToolsError::Json("['namespaces'] has no object".into()))?
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v.as_str().unwrap().to_string()))
            .collect();
        self.wiki = json["wiki"].as_str().map(|s| s.to_string());
        for page_json in json["pages"]
            .as_array()
            .ok_or(ToolsError::Json("['pages'] has no array".into()))?
        {
            let page: PetScanPage = serde_json::from_value(page_json.clone())?;
            self.pages.push(page);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_petscan_new() {
        let ps = PetScan::new(123);
        assert_eq!(ps.psid, 123);
        assert_eq!(ps.pages, vec![]);
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn test_petscan_get_blocking() {
        let mut ps = PetScan::new(25951472);
        ps.run_blocking().unwrap();
        assert_eq!(ps.pages.len(), 1);
        assert_eq!(ps.pages[0].page_id, 3361346);
        assert_eq!(ps.pages[0].page_title, "Magnus_Manske");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pagepile_get_async() {
        let mut ps = PetScan::new(25951472);
        ps.run().await.unwrap();
        assert_eq!(ps.pages.len(), 1);
        assert_eq!(ps.pages[0].page_id, 3361346);
        assert_eq!(ps.pages[0].page_title, "Magnus_Manske");
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn test_petscan_get_blocking_file() {
        let mut ps = PetScan::new(28348161);
        ps.run_blocking().unwrap();
        let expected_giui = PetScanFileUsage {
            ns: 0,
            page: "St._Laurentius_(Wald-Michelbach)".to_string(),
            wiki: "dewiki".to_string(),
        };
        assert!(ps.pages[0].giu.iter().any(|giu| giu == &expected_giui));
        assert!(ps.pages[0].giu.len() > 2);
        assert!(!ps.pages[0].metadata.disambiguation);
        assert_eq!(ps.pages[0].metadata.img_size, 796383);
        assert_eq!(ps.pages[0].metadata.img_height, 1364);
        assert_eq!(ps.pages[0].metadata.img_width, 964);
        assert_eq!(ps.pages[0].page_id, 1166558);
        assert_eq!(
            ps.pages[0].page_title,
            "Germany_wald-michelbach_catholic_church.jpg"
        );
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn test_petscan_get_blocking_metadata() {
        let mut ps = PetScan::new(28348714);
        ps.run_blocking().unwrap();
        assert_eq!(ps.pages[0].page_id, 12115738);
        assert_eq!(ps.pages[0].page_title, "St._Laurentius_(Wald-Michelbach)");
        assert_eq!(
            ps.pages[0].metadata.coordinates(),
            Some((49.572731, 8.82455))
        );
        assert_eq!(
            ps.pages[0].metadata.image,
            "Germany_wald-michelbach_catholic_church.jpg"
        );
        assert_eq!(ps.pages[0].metadata.wikidata, "Q110825193");
    }

    #[test]
    fn test_petscan_into_title() {
        let ps = PetScanPage {
            page_namespace: 0,
            page_title: "Foo".to_string(),
            ..Default::default()
        };
        let title: mediawiki::title::Title = ps.into();
        assert_eq!(title, mediawiki::title::Title::new("Foo", 0));
    }
}
