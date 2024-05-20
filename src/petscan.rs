use std::collections::HashMap;

use crate::ToolsError;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct PetScanFileUsage {
    pub ns: i32,
    pub page: String,
    pub wiki: String,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
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
    pub page_namespace: i32,
    pub page_title: String,
    #[serde(default)]
    pub giu: Vec<PetScanFileUsage>,
    #[serde(default)]
    pub metadata: PetScanMetadata,
}



#[derive(Debug, Default, PartialEq)]
pub struct PetScan {
    psid: u32,
    pages: Vec<PetScanPage>,
    namespaces: HashMap<i32, String>,
    query: Option<String>,
    status: Option<String>,
}

impl PetScan {
    pub fn new(psid: u32) -> Self {
        Self { psid, ..Default::default() }
    }

    #[cfg(feature = "blocking")]
    pub fn get_blocking(&mut self) -> Result<(), ToolsError> {
        let url = format!("https://petscan.wmflabs.org/?psid={psid}&format=json&output_compatability=quick-intersection", psid=self.psid);
        let client = reqwest::blocking::Client::builder()
            .user_agent(crate::TOOLS_INTERFACE_USER_AGENT)
            .build()?;
        let json: Value = client.get(&url).send()?.json()?;
        self.from_json(&json)
    }

    #[cfg(feature = "tokio")]
    pub async fn get(&mut self) -> Result<(), ToolsError> {
        let url = format!("https://petscan.wmflabs.org/?psid={psid}&format=json&output_compatability=quick-intersection", psid=self.psid);
        let client = reqwest::Client::builder()
            .user_agent(crate::TOOLS_INTERFACE_USER_AGENT)
            .build()?;
        let json = client.get(&url).send().await?.json().await?;
        self.from_json(&json)
    }

    fn from_json(&mut self, json: &Value) -> Result<(), ToolsError> {
        self.status = json["status"].as_str().map(|s| s.to_string());
        if self.status!=Some("OK".to_string()) {
            return Err(ToolsError::Tool(format!("PetScan status is not OK: {:?}", self.status)));
        }
        self.query = json["query"].as_str().map(|s| s.to_string());
        self.namespaces = json["namespaces"]
            .as_object()
            .ok_or(ToolsError::Json("['namespaces'] has no object".into()))?
            .iter()
            .map(|(k, v)| (k.parse().unwrap(), v.as_str().unwrap().to_string()))
            .collect();
        for page_json in json["pages"].as_array().ok_or(ToolsError::Json("['pages'] has no array".into()))? {
            let page: PetScanPage = serde_json::from_value(page_json.clone())?;
            self.pages.push(page);
        }
        println!("{:#?}", self);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let ps = PetScan::new(123);
        assert_eq!(ps.psid, 123);
        assert_eq!(ps.pages, vec![]);
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn test_petscan_get_blocking() {
        let mut ps = PetScan::new(25951472);
        ps.get_blocking().unwrap();
        assert_eq!(ps.pages.len(),1);
        assert_eq!(ps.pages[0].page_id, 3361346);
        assert_eq!(ps.pages[0].page_title, "Magnus_Manske");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pagepile_get_tokio() {
        let mut ps = PetScan::new(25951472);
        ps.get().await.unwrap();
        assert_eq!(ps.pages.len(),1);
        assert_eq!(ps.pages[0].page_id, 3361346);
        assert_eq!(ps.pages[0].page_title, "Magnus_Manske");
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn test_petscan_get_blocking_file() {
        let mut ps = PetScan::new(28348161);
        ps.get_blocking().unwrap();
        let expected_giui = PetScanFileUsage { ns: 0, page: "St._Laurentius_(Wald-Michelbach)".to_string(), wiki: "dewiki".to_string() };
        assert!(ps.pages[0].giu.iter().any(|giu| giu==&expected_giui));
        assert!(ps.pages[0].giu.len()>2);
        assert!(!ps.pages[0].metadata.disambiguation);
        assert_eq!(ps.pages[0].metadata.img_size, 796383);
        assert_eq!(ps.pages[0].metadata.img_height, 1364);
        assert_eq!(ps.pages[0].metadata.img_width, 964);
        assert_eq!(ps.pages[0].page_id, 1166558);
        assert_eq!(ps.pages[0].page_title, "Germany_wald-michelbach_catholic_church.jpg");
        assert_eq!(ps.pages[0].metadata.coordinates(), Some((49.572731,8.82455)));
    }
}
