use crate::fancy_title::FancyTitle;
use crate::{Site, ToolsError, ToolsInterface};
use mediawiki::api::Api;
use mediawiki::title::Title;
use serde_json::{self, Map, Value, json};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Clone)]
pub struct Page {
    title: Title,
    meta: Map<String, Value>,
}

impl Page {
    pub fn as_json(&self, api: &Api) -> Value {
        let mut json =
            FancyTitle::new(self.title.pretty(), self.title.namespace_id(), api).to_json();
        self.meta.iter().for_each(|(key, value)| {
            json[key] = value.clone();
        });
        json
    }

    fn key(&self) -> String {
        format!(
            "{}:{}",
            self.title.namespace_id(),
            self.title.with_underscores()
        )
    }

    fn merge(&self, other: &Page) -> Page {
        let mut meta = self.meta.clone();
        meta.extend(other.meta.clone());
        Page {
            title: self.title.clone(),
            meta,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageList {
    pages: Vec<Page>,
    site: Site,
}

impl PageList {
    pub fn from_json(json: &Value) -> Result<Self, ToolsError> {
        let wiki = json["site"]["wiki"]
            .as_str()
            .ok_or(ToolsError::Json("missing site.wiki".to_string()))?;
        let site = Site::from_wiki(wiki)
            .ok_or_else(|| ToolsError::Tool(format!("Unknown wiki {wiki}")))?;

        let pages = json["pages"]
            .as_array()
            .ok_or(ToolsError::Json("missing pages".to_string()))?;
        let pages = pages
            .iter()
            .filter_map(|page| {
                let mut meta = page.as_object()?.to_owned();
                let title = page["title"].as_str()?;
                let namespace_id = page["namespace_id"].as_i64()?;
                let _ = meta.remove("title")?;
                let _ = meta.remove("namespace_id")?;
                let _ = meta.remove("prefixed_title")?;
                let title = Title::new(title, namespace_id);
                Some(Page { title, meta })
            })
            .collect();
        Ok(Self { pages, site })
    }

    pub fn from_file(filename: &str) -> Result<Self, ToolsError> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        let json = serde_json::from_reader(reader)?;
        Self::from_json(&json)
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub async fn as_json(&self) -> Value {
        let site = self.site();
        let api = site.api().await.ok().unwrap();
        json!({
            "pages": self.pages()
                .iter()
                .map(|page| page.as_json(&api))
                .collect::<Vec<Value>>(),
            "site": site,
        })
    }

    pub async fn to_wiki(&self, target_wiki: &str) -> Self {
        let api = self.site.api().await.ok().unwrap();
        let source_wiki = self.site.wiki();
        let url =
            format!("https://wd-infernal.toolforge.org/change_wiki/{source_wiki}/{target_wiki}");
        let pages: Vec<String> = self
            .pages
            .iter()
            .filter_map(|page| page.title.full_pretty(&api))
            .collect();
        let payload = json!(pages);
        let client = ToolsInterface::tokio_client().unwrap();
        let old2new: HashMap<String, String> = client
            .post(url) // Replace with your URL
            .json(&payload) // Set the JSON payload
            .send() // Send the request
            .await // Await the response
            .unwrap()
            .json()
            .await
            .unwrap();

        let mut ret = Self {
            site: Site::from_wiki(target_wiki).unwrap(),
            pages: Vec::new(),
        };
        for page in &self.pages {
            let title = page.title.full_pretty(&api).unwrap();
            if let Some(new_title) = old2new.get(&title) {
                let mut new_page = page.clone();
                new_page.title = Title::new_from_full(new_title, &api);
                ret.pages.push(new_page);
            }
        }

        ret
    }

    pub async fn subset(&self, other: &Self) -> Self {
        let mut other = other.to_owned();
        // Convert to same wiki, if necessary
        if self.site != other.site {
            other = other.to_wiki(self.site.wiki()).await;
        }

        let title2pos = other
            .pages
            .iter()
            .enumerate()
            .map(|(i, page)| (page.key(), i))
            .collect::<HashMap<String, usize>>();
        let pages = self
            .pages
            .iter()
            .filter(|page| title2pos.contains_key(&page.key()))
            .map(|page| page.merge(&other.pages[title2pos[&page.key()]]))
            .collect();
        Self {
            pages,
            site: self.site.clone(),
        }
    }

    pub async fn union(&self, other: &Self) -> Self {
        let mut other = other.to_owned();
        // Convert to same wiki, if necessary
        if self.site != other.site {
            other = other.to_wiki(self.site.wiki()).await;
        }

        // Get unique and merged pages from this set
        let mut title2pos = other
            .pages
            .iter()
            .enumerate()
            .map(|(i, page)| (page.key(), i))
            .collect::<HashMap<String, usize>>();
        let mut pages: Vec<Page> = self
            .pages
            .iter()
            .map(|page| match title2pos.remove(&page.key()) {
                Some(pos) => page.merge(&other.pages[pos]),
                None => page.clone(),
            })
            .collect();

        // Add the missing pages from other
        let other_pages = title2pos
            .values()
            .map(|&pos| other.pages[pos].clone())
            .collect::<Vec<_>>();
        pages.extend(other_pages);

        Self {
            pages,
            site: self.site.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subset() {
        let pl1 = PageList::from_file("test_data/pagelist1.json").unwrap();
        let pl2 = PageList::from_file("test_data/pagelist2.json").unwrap();
        let pl3 = pl1.subset(&pl2).await;
        assert_eq!(pl3.pages.len(), 1);
    }

    #[tokio::test]
    async fn test_union() {
        let pl1 = PageList::from_file("test_data/pagelist1.json").unwrap();
        let pl2 = PageList::from_file("test_data/pagelist2.json").unwrap();
        let pl3 = pl1.union(&pl2).await;
        assert_eq!(pl3.pages.len(), 12);
    }

    #[tokio::test]
    async fn test_to_wiki() {
        let pl = PageList {
            site: Site::from_wiki("enwiki").unwrap(),
            pages: vec![
                Page {
                    title: Title::new("Biochemistry", 0),
                    meta: Map::new(),
                },
                Page {
                    title: Title::new("Magnus_Manske", 0),
                    meta: Map::new(),
                },
            ],
        };
        let pl2 = pl.to_wiki("dewiki").await;
        assert_eq!(pl2.pages.len(), 2);
        assert_eq!(pl2.pages[0].title.pretty(), "Biochemie");
        assert_eq!(pl2.pages[1].title.pretty(), "Magnus Manske");
    }
}
