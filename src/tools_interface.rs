use std::time::Duration;
use mediawiki::api::Api;
use std::collections::HashMap;
use crate::ToolsError;

const DEFAULT_CLIENT_TIMEOUT_SECONDS: u64 = 60;

pub static TOOLS_INTERFACE_USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub struct ToolsInterface {}

impl ToolsInterface {
    #[cfg(feature = "blocking")]
    pub fn blocking_client() -> Result<reqwest::blocking::Client, ToolsError> {
        Ok(reqwest::blocking::Client::builder()
            .user_agent(crate::TOOLS_INTERFACE_USER_AGENT)
            .timeout(Duration::from_secs(DEFAULT_CLIENT_TIMEOUT_SECONDS))
            .build()?)
    }

    #[cfg(feature = "tokio")]
    pub fn tokio_client() -> Result<reqwest::Client, ToolsError> {
        Ok(reqwest::Client::builder()
            .user_agent(crate::TOOLS_INTERFACE_USER_AGENT)
            .timeout(Duration::from_secs(DEFAULT_CLIENT_TIMEOUT_SECONDS))
            .build()?)
    }

    #[cfg(feature = "tokio")]
    /// Returns a MediaWiki API object for Wikidata.
    pub async fn wikidata_api() -> Result<Api, ToolsError> {
        let api = Api::new("https://www.wikidata.org/w/api.php").await?;
        Ok(api)
    }

    #[cfg(feature = "tokio")]
    /// Returns a MediaWiki API object for Wikimedia Commons.
    pub async fn commons_api() -> Result<Api, ToolsError> {
        let api = Api::new("https://commons.wikimedia.org/w/api.php").await?;
        Ok(api)
    }

    #[cfg(feature = "tokio")]
    /// Takes a wiki and a list of prefixed titles.
    /// Returns a map of titles (spaces, not underscores) to Wikidata IDs.
    pub async fn wikidata_item_for_titles(
        wiki: &str,
        titles: &[String],
    ) -> Result<HashMap<String, String>, ToolsError> {
        use futures::stream::StreamExt;

        const MAX_CONCURRENT: usize = 5;

        let api_params =
            Self::generate_api_params_for_wikidata_item_for_titles(titles, wiki).await?;
        let futures = api_params
            .iter()
            .map(|(api, params)| api.get_query_api_json(params));
        let stream = futures::stream::iter(futures).buffered(MAX_CONCURRENT);
        let results = stream.collect::<Vec<_>>().await;
        let mut ret = HashMap::new();
        for result in results {
            let result = result?;
            let entities = result["entities"]
                .as_object()
                .ok_or_else(|| ToolsError::Json("['entities'] is not an object".into()))?;
            for (id, v) in entities.iter() {
                let sitelinks = v
                    .get("sitelinks")
                    .ok_or_else(|| ToolsError::Json("['sitelinks'] does not exist".into()))?
                    .as_object()
                    .ok_or_else(|| ToolsError::Json("['sitelinks'] is not an object".into()))?;
                let sitelink = sitelinks
                    .get(wiki)
                    .ok_or_else(|| ToolsError::Json("site link not found".into()))?;
                let title = sitelink
                    .get("title")
                    .ok_or_else(|| ToolsError::Json("['title'] does not exist".into()))?
                    .as_str()
                    .ok_or_else(|| ToolsError::Json("['title'] is not a string".into()))?;
                ret.insert(title.replace('_', " ").to_string(), id.to_string());
            }
        }
        Ok(ret)
    }

    async fn generate_api_params_for_wikidata_item_for_titles(
        titles: &[String],
        wiki: &str,
    ) -> Result<Vec<(std::sync::Arc<Api>, HashMap<String, String>)>, ToolsError> {
        use std::sync::Arc;
        let api = Arc::new(Self::wikidata_api().await?);
        let api_params: Vec<_> = titles
            .chunks(50)
            .map(|chunk| {
                let chunk = chunk.join("|");
                let params: HashMap<String, String> = [
                    ("action", "wbgetentities"),
                    ("format", "json"),
                    ("sites", wiki),
                    ("titles", &chunk),
                    ("props", "sitelinks"), // return only sitelinks...
                    ("sitefilter", &wiki),  // ...from this wiki
                ]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
                (api.clone(), params)
            })
            .collect();
        Ok(api_params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_wikidata_item_for_titles() {
        let wiki = "dewiki";
        let titles = vec![
            "Albert Einstein".to_string(),
            "Isaac Newton".to_string(),
            "Johannes Kepler".to_string(),
        ];

        let result = ToolsInterface::wikidata_item_for_titles(wiki, &titles)
            .await
            .unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result.get("Albert Einstein"), Some(&"Q937".to_string()));
        assert_eq!(result.get("Isaac Newton"), Some(&"Q935".to_string()));
        assert_eq!(result.get("Johannes Kepler"), Some(&"Q8963".to_string()));
    }
}
