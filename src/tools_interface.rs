/// # ToolsInterface
/// Some common helper functions for interacting with the Wikimedia Toolforge environment,
/// e.g. creating HTTP clients, getting Wikidata item ID for page titles, etc.
///
/// ## Example
/// ```ignore
/// let result = ToolsInterface::wikidata_item_for_titles(wiki, ["Albert Einstein".to_string()]).await.unwrap();
/// let q = result.get("Albert Einstein").unwrap(); // Yields "Q937"
/// ```
use crate::ToolsError;
use mediawiki::api::Api;
use std::collections::HashMap;
use std::time::Duration;

const DEFAULT_CLIENT_TIMEOUT_SECONDS: u64 = 300; // 5min

pub static TOOLS_INTERFACE_USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

pub struct ToolsInterface {}

impl ToolsInterface {
    #[cfg(feature = "blocking")]
    pub fn blocking_client() -> Result<reqwest::blocking::Client, ToolsError> {
        Ok(reqwest::blocking::Client::builder()
            .user_agent(TOOLS_INTERFACE_USER_AGENT)
            .timeout(Duration::from_secs(DEFAULT_CLIENT_TIMEOUT_SECONDS))
            .build()?)
    }

    #[cfg(feature = "tokio")]
    pub fn tokio_client() -> Result<reqwest::Client, ToolsError> {
        Ok(reqwest::Client::builder()
            .user_agent(TOOLS_INTERFACE_USER_AGENT)
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

    #[cfg(feature = "tokio")]
    /// Maps page titles on `from_wiki` to the titles of the same topic on `to_wiki`,
    /// using Wikidata sitelinks. Titles without a counterpart are absent from the result.
    ///
    /// `wikidatawiki` is accepted on either side, where the "title" of a page is its entity ID.
    pub async fn sitelinks_between_wikis(
        from_wiki: &str,
        to_wiki: &str,
        titles: &[String],
    ) -> Result<HashMap<String, String>, ToolsError> {
        use futures::stream::StreamExt;

        const MAX_CONCURRENT: usize = 5;
        const MAX_TITLES_PER_REQUEST: usize = 50;

        if from_wiki == to_wiki {
            return Ok(titles
                .iter()
                .map(|title| (title.to_owned(), title.to_owned()))
                .collect());
        }
        let api = std::sync::Arc::new(Self::wikidata_api().await?);
        let requests: Vec<_> = titles
            .chunks(MAX_TITLES_PER_REQUEST)
            .map(|chunk| {
                let mut params: HashMap<String, String> = [
                    ("action", "wbgetentities"),
                    ("format", "json"),
                    ("props", "sitelinks"),
                ]
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect();
                // Wikidata pages are addressed by entity ID, not by sitelink.
                if from_wiki == "wikidatawiki" {
                    params.insert("ids".to_string(), chunk.join("|"));
                } else {
                    params.insert("sites".to_string(), from_wiki.to_string());
                    params.insert("titles".to_string(), chunk.join("|"));
                }
                // The source sitelink is needed to map the result back to the input title.
                params.insert(
                    "sitefilter".to_string(),
                    [from_wiki, to_wiki]
                        .iter()
                        .filter(|wiki| **wiki != "wikidatawiki")
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("|"),
                );
                (api.clone(), params)
            })
            .collect();

        let futures = requests
            .iter()
            .map(|(api, params)| api.get_query_api_json(params));
        let results = futures::stream::iter(futures)
            .buffered(MAX_CONCURRENT)
            .collect::<Vec<_>>()
            .await;

        let mut ret = HashMap::new();
        for result in results {
            let entities = result?["entities"]
                .as_object()
                .ok_or_else(|| ToolsError::Json("['entities'] is not an object".into()))?
                .to_owned();
            for (id, entity) in entities.iter() {
                let sitelinks = entity["sitelinks"].as_object();
                let title_on = |wiki: &str| -> Option<String> {
                    if wiki == "wikidatawiki" {
                        return Some(id.to_owned());
                    }
                    Some(
                        sitelinks?.get(wiki)?["title"]
                            .as_str()?
                            .replace('_', " ")
                            .to_string(),
                    )
                };
                if let (Some(from), Some(to)) = (title_on(from_wiki), title_on(to_wiki)) {
                    ret.insert(from, to);
                }
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
                    ("sitefilter", wiki),   // ...from this wiki
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
    async fn test_sitelinks_between_wikis() {
        let titles = vec!["Biochemistry".to_string(), "Isaac Newton".to_string()];
        let map = ToolsInterface::sitelinks_between_wikis("enwiki", "dewiki", &titles)
            .await
            .unwrap();
        assert_eq!(map.get("Biochemistry"), Some(&"Biochemie".to_string()));
        assert_eq!(map.get("Isaac Newton"), Some(&"Isaac Newton".to_string()));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_sitelinks_to_and_from_wikidata() {
        let titles = vec!["Isaac Newton".to_string()];
        let to_wikidata =
            ToolsInterface::sitelinks_between_wikis("enwiki", "wikidatawiki", &titles)
                .await
                .unwrap();
        assert_eq!(to_wikidata.get("Isaac Newton"), Some(&"Q935".to_string()));

        let items = vec!["Q935".to_string()];
        let from_wikidata =
            ToolsInterface::sitelinks_between_wikis("wikidatawiki", "dewiki", &items)
                .await
                .unwrap();
        assert_eq!(from_wikidata.get("Q935"), Some(&"Isaac Newton".to_string()));
    }

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
