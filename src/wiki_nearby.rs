/// # WikiNearby
/// Module for interacting with the [WikiNearby](https://wikinearby.toolforge.org) tool.
/// You can retrieve a list of pages on one wiki that are nearby a set of coordinates
/// You can query either by coordinates or by a page title.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let site = Site::from_wiki("enwiki").unwrap();
/// let title = "Cambridge";
/// let mut a = WikiNearby::new(site, title);
/// a.run().await.unwrap();
/// a.results()
///     .iter()
///     .for_each(|result| {
///        println!("Page {} Description {} Lat {} Lon {} Image {}", result.title, result.description, result.lat, result.lon, result.image);
///     });
/// ```
use crate::{Site, Tool, ToolsError, fancy_title::FancyTitle};
use async_trait::async_trait;
use serde_json::{Value, json};

#[derive(Debug, Default, PartialEq)]
pub struct WikiNearbyResult {
    pub title: String,
    pub description: String,
    pub lat: f64,
    pub lon: f64,
    pub distance: f64, // km
    pub image: Option<String>,
}

impl WikiNearbyResult {
    fn from_json(entry: &Value) -> Option<Self> {
        Some(WikiNearbyResult {
            title: entry["page"].as_str()?.to_string(),
            description: entry["desc"].as_str()?.to_string(),
            image: entry["img"].as_str().map(|image| image.to_string()),
            lat: Self::json2f64(&entry["lat"])?,
            lon: Self::json2f64(&entry["lon"])?,
            distance: Self::json2f64(&entry["dist"])?,
        })
    }

    fn json2f64(j: &Value) -> Option<f64> {
        j.as_str()?.parse().ok()
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct WikiNearby {
    site: Site,
    query: String,
    offset: usize,
    results: Vec<WikiNearbyResult>,
    lat: Option<f64>,
    lon: Option<f64>,
}

impl WikiNearby {
    pub fn new_from_page(site: Site, title: &str) -> Self {
        Self {
            site,
            query: title.to_string(),
            ..Default::default()
        }
    }

    pub fn new_from_coordinates(site: Site, lat: f64, lon: f64) -> Self {
        Self {
            site,
            query: format!("{lat}, {lon}"),
            ..Default::default()
        }
    }

    pub fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn results(&self) -> &[WikiNearbyResult] {
        &self.results
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    /// Returns the latitude of the article from the API call
    pub fn lat(&self) -> Option<f64> {
        self.lat
    }

    /// Returns the longitude of the article from the API call
    pub fn lon(&self) -> Option<f64> {
        self.lon
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub async fn as_json(&self) -> Value {
        let site = self.site();
        let api = site.api().await.unwrap();
        json!({
            "pages": self.results()
                .iter()
                .map(|result| (FancyTitle::from_prefixed(&result.title, &api).to_json(),result))
                .map(|(mut v,result)| {
                v["image"] = json!(result.image);
                v["lat"] = json!(result.lat);
                v["lon"] = json!(result.lon);
                v["distance"] = json!(result.distance);
                v
                })
                .collect::<Vec<Value>>(),
            "site": site,
        })
    }
}

#[async_trait]
impl Tool for WikiNearby {
    fn get_url(&self) -> String {
        format!(
            "https://wikinearby.toolforge.org/api/nearby?q={query}&lang={lang}&offset={offset}",
            query = self.query,
            lang = self.site.language(),
            offset = self.offset,
        )
    }

    fn set_from_json(&mut self, j: Value) -> Result<(), ToolsError> {
        self.lat = WikiNearbyResult::json2f64(&j["lat"]);
        self.lon = WikiNearbyResult::json2f64(&j["lon"]);
        for entry in j["list"]
            .as_array()
            .ok_or_else(|| ToolsError::Json("Result is not an array".to_string()))?
        {
            match WikiNearbyResult::from_json(entry) {
                Some(result) => self.results.push(result),
                None => continue,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_from_page() {
        let site = Site::from_wiki("enwiki").unwrap();
        let title = "Cambridge";
        let tool = WikiNearby::new_from_page(site.clone(), title);
        assert_eq!(tool.site(), &site);
        assert_eq!(tool.query(), title);
    }

    #[test]
    fn test_new_from_coordinates() {
        let site = Site::from_wiki("enwiki").unwrap();
        let tool = WikiNearby::new_from_coordinates(site.clone(), 52.205, 0.1225);
        assert_eq!(tool.site(), &site);
        assert_eq!(tool.query(), "52.205, 0.1225");
    }

    #[tokio::test]
    async fn test_json() {
        let site = Site::from_wiki("enwiki").unwrap();
        let title = "Cambridge";
        let mut tool = WikiNearby::new_from_page(site, title);
        tool.run().await.unwrap();
        assert!(
            tool.results()
                .iter()
                .any(|result| result.distance == 0.12 && result.title == "Grand_Arcade_(Cambridge)")
        );
    }
}
