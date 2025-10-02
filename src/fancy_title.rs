use mediawiki::{api::Api, title::Title};
use serde_json::json;

#[derive(Debug, PartialEq)]
pub struct FancyTitle {
    pub title: Title,
    pub prefixed_title: String,
}

impl FancyTitle {
    pub fn new(s: &str, ns: i64, api: &Api) -> Self {
        let title = Title::new(s, ns);
        Self {
            prefixed_title: title.full_pretty(api).unwrap_or_default(),
            title,
        }
    }

    pub fn from_prefixed(s: &str, api: &Api) -> Self {
        let title = Title::new_from_full(s, api);
        Self {
            prefixed_title: title.full_pretty(api).unwrap_or_default(),
            title,
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "title": self.title.pretty(),
            "prefixed_title": self.prefixed_title,
            "namespace_id": self.title.namespace_id(),
        })
    }
}
