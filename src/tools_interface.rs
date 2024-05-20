use std::collections::HashMap;
use mediawiki::api::Api;
use crate::ToolsError;

pub static TOOLS_INTERFACE_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);

pub struct ToolsInterface {

}

impl ToolsInterface {
    pub fn blocking_client() -> Result<reqwest::blocking::Client,ToolsError> {
        Ok(reqwest::blocking::Client::builder()
            .user_agent(crate::TOOLS_INTERFACE_USER_AGENT)
            .build()?)
    }

    pub fn tokio_client() -> Result<reqwest::Client,ToolsError> {
        Ok(reqwest::Client::builder()
            .user_agent(crate::TOOLS_INTERFACE_USER_AGENT)
            .build()?)
    }
    
    pub async fn wikidata_api() -> Result<Api,ToolsError> {
        let api = Api::new("https://www.wikidata.org/w/api.php").await?;
        Ok(api)
    }

    #[cfg(feature = "tokio")]
    /// Takes a wiki and a list of (perfectly formatted) titles.
    /// Returns a map of titles to Wikidata IDs.
    pub async fn wikidata_item_for_titles(wiki: &str, titles: &[String]) -> Result<HashMap<String,String>,ToolsError> {
        use std::sync::Arc;
        use futures::stream::StreamExt;

        const MAX_CONCURRENT: usize = 5;

        let api = Arc::new(Self::wikidata_api().await?);
        let api_params = Self::generate_api_params(titles, wiki, api);
        let futures = api_params.iter().map(|(api,params)| api.get_query_api_json(params));
        let mut ret = HashMap::new();
        let stream = futures::stream::iter(futures).buffered(MAX_CONCURRENT);
        let results = stream.collect::<Vec<_>>().await;
        for result in results {
            let result = result?;
            for (id,v) in result["entities"].as_object().ok_or(ToolsError::Json("['entities'] is not an object".into()))?.iter() {
                let sitelinks = v.get("sitelinks")
                    .ok_or(ToolsError::Json("['sitelinks'] does not exist".into()))?
                    .as_object()
                    .ok_or(ToolsError::Json("['sitelinks'] is not an object".into()))?;
                let sitelink = sitelinks.get(wiki).ok_or(ToolsError::Json("site link not found".into()))?;
                let title = sitelink.get("title")
                    .ok_or(ToolsError::Json("['title'] does not exist".into()))?
                    .as_str().ok_or(ToolsError::Json("['title'] is not a string".into()))?;
                ret.insert(title.to_string(),id.to_string());
            }
        }
        Ok(ret)
    }

    fn generate_api_params(titles: &[String], wiki: &str, api: std::sync::Arc<Api>) -> Vec<(std::sync::Arc<Api>, HashMap<String, String>)> {
        let api_params: Vec<_> = titles.chunks(50)
            .map(|chunk| {
                let chunk = chunk.join("|");
                let params: HashMap<String,String> = [
                        ("action","wbgetentities"),
                        ("format","json"),
                        ("sites",wiki),
                        ("titles",&chunk),
                        ("props","sitelinks"), // return only sitelinks...
                        ("sitefilter",&wiki), // ...from this wiki
                    ].iter()
                    .map(|(k,v)| (k.to_string(),v.to_string()))
                    .collect();
                (api.clone(),params)
            })
            .collect();
        api_params
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

        let result = ToolsInterface::wikidata_item_for_titles(wiki,&titles).await.unwrap();
        assert_eq!(result.len(),3);
        assert_eq!(result.get("Albert Einstein"),Some(&"Q937".to_string()));
        assert_eq!(result.get("Isaac Newton"),Some(&"Q935".to_string()));
        assert_eq!(result.get("Johannes Kepler"),Some(&"Q8963".to_string()));
    }
}