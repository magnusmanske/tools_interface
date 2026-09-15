//! # PageList
//! A `PageList` is a list of pages on a single wiki, plus whatever metadata the
//! tools that produced it attached to each page.
//! It is the interchange format that makes tool output composable:
//! lists can be unioned, intersected, subtracted, and cast to another wiki.
//!
//! ## File format
//! Lists are read from and written to JSONL (one JSON object per line), so they can be
//! piped between processes and filtered with `jq`.
//!
//! The first line is an optional header carrying per-list metadata:
//! ```json
//! {"ti":1,"site":{"wiki":"enwiki","language":"en","project":"wikipedia"},"sources":[…]}
//! ```
//!
//! Every following line is one page:
//! ```json
//! {"wiki":"enwiki","namespace_id":0,"title":"Earth","prefixed_title":"Earth","meta":{"petscan":{…}}}
//! ```
//!
//! Each page repeats its `wiki`, so a list survives `jq` filtering that drops the header
//! line. Only the `sources` provenance is lost in that case.
//! Tool-specific metadata is namespaced by tool name under `meta`, so metadata from
//! different tools can never overwrite each other when lists are merged.
//!
//! The pre-0.2.0 format (a single JSON document with a `pages` array) is still read.

use crate::{Site, ToolsError, ToolsInterface};
use mediawiki::api::Api;
use mediawiki::title::Title;
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

/// Version of the JSONL container format written by [`PageList::to_writer`].
pub const FORMAT_VERSION: u64 = 1;

/// Metadata namespace used for pages read from the pre-0.2.0 format, where
/// per-page metadata was not attributed to a tool.
pub const LEGACY_META_KEY: &str = "_extra";

/// Metadata namespace recording where a page came from before [`PageList::cast`].
pub const CAST_META_KEY: &str = "_cast";

/// Page fields that describe identity rather than tool-specific metadata.
const IDENTITY_KEYS: [&str; 4] = ["wiki", "title", "prefixed_title", "namespace_id"];

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    title: Title,
    meta: Map<String, Value>,
}

impl Page {
    pub fn new(title: Title) -> Self {
        Self {
            title,
            meta: Map::new(),
        }
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    /// All metadata, keyed by the tool that contributed it.
    pub fn meta(&self) -> &Map<String, Value> {
        &self.meta
    }

    /// The metadata contributed by a single tool, if any.
    pub fn meta_for(&self, tool: &str) -> Option<&Value> {
        self.meta.get(tool)
    }

    pub fn set_meta_for(&mut self, tool: &str, value: Value) {
        self.meta.insert(tool.to_string(), value);
    }

    /// Identity of the page within its wiki. Pages with equal keys are the same page.
    fn key(&self) -> String {
        format!(
            "{}:{}",
            self.title.namespace_id(),
            self.title.with_underscores()
        )
    }

    /// Combines the metadata of both pages. Metadata from different tools is kept
    /// side by side; within one tool's namespace, `other` wins on conflicting keys.
    fn merge(&self, other: &Page) -> Page {
        let mut meta = self.meta.clone();
        for (tool, value) in &other.meta {
            match (meta.get_mut(tool), value.as_object()) {
                (Some(Value::Object(existing)), Some(incoming)) => {
                    existing.extend(incoming.iter().map(|(k, v)| (k.clone(), v.clone())));
                }
                _ => {
                    meta.insert(tool.clone(), value.clone());
                }
            }
        }
        Page {
            title: self.title.clone(),
            meta,
        }
    }

    /// Reads a page in the JSONL record format.
    fn from_record(value: &Value) -> Result<Self, ToolsError> {
        let title = value["title"]
            .as_str()
            .ok_or_else(|| ToolsError::Json(format!("page has no title: {value}")))?;
        let namespace_id = value["namespace_id"]
            .as_i64()
            .ok_or_else(|| ToolsError::Json(format!("page has no namespace_id: {value}")))?;
        let meta = match value.get("meta") {
            Some(Value::Object(meta)) => meta.clone(),
            _ => Map::new(),
        };
        Ok(Self {
            title: Title::new(title, namespace_id),
            meta,
        })
    }

    /// Reads a page from a tool's own JSON output, where metadata sits at the top
    /// level alongside the identity fields. Those extra fields are moved into
    /// `meta[tool]`, so they cannot collide with another tool's metadata.
    fn from_tool_json(value: &Value, tool: &str) -> Result<Self, ToolsError> {
        let mut page = Self::from_record(value)?;
        let extra: Map<String, Value> = value
            .as_object()
            .map(|object| {
                object
                    .iter()
                    .filter(|(key, _)| !IDENTITY_KEYS.contains(&key.as_str()) && *key != "meta")
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if !extra.is_empty() {
            page.set_meta_for(tool, Value::Object(extra));
        }
        Ok(page)
    }

    /// The JSONL record for this page. `api` is only needed to render `prefixed_title`
    /// for pages outside the main namespace.
    fn to_record(&self, wiki: &str, api: Option<&Api>) -> Value {
        let mut json = json!({
            "wiki": wiki,
            "namespace_id": self.title.namespace_id(),
            "title": self.title.pretty(),
        });
        if let Some(prefixed_title) = self.prefixed_title(api) {
            json["prefixed_title"] = json!(prefixed_title);
        }
        if !self.meta.is_empty() {
            json["meta"] = Value::Object(self.meta.clone());
        }
        json
    }

    /// The namespace-prefixed title. Returns `None` if the namespace name is not known,
    /// which can only happen for non-main namespaces when no `Api` is available.
    fn prefixed_title(&self, api: Option<&Api>) -> Option<String> {
        match api {
            Some(api) => self.title.full_pretty(api),
            None if self.title.namespace_id() == 0 => Some(self.title.pretty().to_string()),
            None => None,
        }
    }

    /// The pre-0.2.0 page representation, with metadata flattened to the top level.
    fn to_legacy_json(&self, api: Option<&Api>) -> Value {
        let mut json = json!({
            "title": self.title.pretty(),
            "prefixed_title": self.prefixed_title(api).unwrap_or_default(),
            "namespace_id": self.title.namespace_id(),
        });
        for value in self.meta.values() {
            if let Some(object) = value.as_object() {
                for (key, value) in object {
                    json[key] = value.clone();
                }
            }
        }
        json
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageList {
    site: Site,
    pages: Vec<Page>,
    sources: Vec<Value>,
}

impl PageList {
    pub fn new(site: Site) -> Self {
        Self {
            site,
            pages: Vec::new(),
            sources: Vec::new(),
        }
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn pages_mut(&mut self) -> &mut Vec<Page> {
        &mut self.pages
    }

    /// Provenance: which tools and operations produced this list.
    pub fn sources(&self) -> &[Value] {
        &self.sources
    }

    pub fn add_source(&mut self, source: Value) {
        self.sources.push(source);
    }

    pub fn len(&self) -> usize {
        self.pages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    // MARK: Reading

    /// Reads a list in either the JSONL format or the pre-0.2.0 JSON format.
    ///
    /// `wiki_override` forces the list's wiki, which is required for headerless input
    /// whose pages carry no `wiki` field, and which also suppresses the mixed-wiki check.
    pub fn from_reader<R: BufRead>(
        reader: R,
        wiki_override: Option<&str>,
    ) -> Result<Self, ToolsError> {
        let mut lines = reader.lines();
        let mut first = None;
        for line in lines.by_ref() {
            let line = line?;
            if !line.trim().is_empty() {
                first = Some(line);
                break;
            }
        }
        let Some(first) = first else {
            return Err(ToolsError::Json("input is empty".into()));
        };

        // A pretty-printed pre-0.2.0 document starts with a line containing only "{".
        if first.trim() == "{" {
            let mut document = first;
            for line in lines {
                document.push('\n');
                document.push_str(&line?);
            }
            return Self::from_json(&serde_json::from_str(&document)?);
        }

        let mut records = vec![serde_json::from_str::<Value>(&first)?];
        // A pre-0.2.0 document written on a single line.
        if records[0].get("pages").is_some_and(Value::is_array) {
            return Self::from_json(&records[0]);
        }
        for line in lines {
            let line = line?;
            if !line.trim().is_empty() {
                records.push(serde_json::from_str(&line)?);
            }
        }
        Self::from_records(&records, wiki_override)
    }

    fn from_records(records: &[Value], wiki_override: Option<&str>) -> Result<Self, ToolsError> {
        let mut site = match wiki_override {
            Some(wiki) => Some(Self::site_for_wiki(wiki)?),
            None => None,
        };
        let mut sources = Vec::new();
        let mut pages = Vec::new();
        for record in records {
            if Self::is_header(record) {
                if let Some(header_sources) = record["sources"].as_array() {
                    sources.extend(header_sources.iter().cloned());
                }
                if wiki_override.is_none() {
                    let wiki = record["site"]["wiki"]
                        .as_str()
                        .ok_or_else(|| ToolsError::Json("header has no site.wiki".into()))?;
                    site = Some(Self::site_for_wiki(wiki)?);
                }
                continue;
            }
            if let (None, Some(wiki)) = (&wiki_override, record["wiki"].as_str()) {
                let page_site = Self::site_for_wiki(wiki)?;
                match &site {
                    None => site = Some(page_site),
                    Some(site) if *site != page_site => {
                        return Err(ToolsError::Tool(format!(
                            "list mixes wikis ({} and {}); use --wiki to force one",
                            site.wiki(),
                            page_site.wiki()
                        )));
                    }
                    Some(_) => {}
                }
            }
            pages.push(Page::from_record(record)?);
        }
        let site = site.ok_or_else(|| {
            ToolsError::Tool(
                "cannot determine the wiki: input has no header and no page \"wiki\" field; pass --wiki"
                    .into(),
            )
        })?;
        Ok(Self {
            site,
            pages,
            sources,
        })
    }

    /// The header line is the only record without a `title`.
    fn is_header(record: &Value) -> bool {
        record.get("ti").is_some()
            || (record.get("title").is_none() && record.get("site").is_some())
    }

    fn site_for_wiki(wiki: &str) -> Result<Site, ToolsError> {
        Site::from_wiki(wiki).ok_or_else(|| ToolsError::Tool(format!("Unknown wiki {wiki}")))
    }

    pub fn from_file(filename: &str) -> Result<Self, ToolsError> {
        Self::from_path(filename, None)
    }

    /// Reads a list from a file, or from stdin if `filename` is `-`.
    pub fn from_path(filename: &str, wiki_override: Option<&str>) -> Result<Self, ToolsError> {
        match filename {
            "-" => Self::from_reader(BufReader::new(std::io::stdin().lock()), wiki_override),
            _ => Self::from_reader(BufReader::new(File::open(filename)?), wiki_override),
        }
    }

    /// Reads a list in the pre-0.2.0 format: a single JSON document with a `pages` array.
    pub fn from_json(json: &Value) -> Result<Self, ToolsError> {
        Self::from_tool_json(json, LEGACY_META_KEY)
    }

    /// Reads a list from a tool's own JSON output, attributing each page's metadata to `tool`.
    pub fn from_tool_json(json: &Value, tool: &str) -> Result<Self, ToolsError> {
        let wiki = json["site"]["wiki"]
            .as_str()
            .ok_or_else(|| ToolsError::Json("missing site.wiki".to_string()))?;
        let pages = json["pages"]
            .as_array()
            .ok_or_else(|| ToolsError::Json("missing pages".to_string()))?
            .iter()
            .map(|page| Page::from_tool_json(page, tool))
            .collect::<Result<Vec<Page>, ToolsError>>()?;
        Ok(Self {
            site: Self::site_for_wiki(wiki)?,
            pages,
            sources: Vec::new(),
        })
    }

    // MARK: Writing

    /// Writes the list as JSONL: a header line, then one line per page.
    pub async fn to_writer<W: Write>(&self, out: &mut W) -> Result<(), ToolsError> {
        let api = self.api_for_prefixed_titles().await;
        let header = json!({
            "ti": FORMAT_VERSION,
            "site": self.site,
            "sources": self.sources,
        });
        writeln!(out, "{}", serde_json::to_string(&header)?)?;
        for page in &self.pages {
            let record = page.to_record(self.site.wiki(), api.as_ref());
            writeln!(out, "{}", serde_json::to_string(&record)?)?;
        }
        out.flush()?;
        Ok(())
    }

    /// The namespace-prefixed titles of all pages.
    pub async fn prefixed_titles(&self) -> Vec<String> {
        let api = self.api_for_prefixed_titles().await;
        self.pages
            .iter()
            .filter_map(|page| page.prefixed_title(api.as_ref()))
            .collect()
    }

    /// The namespace-prefixed titles of all pages, failing if any of them cannot be
    /// rendered. Use this where silently dropping pages would be wrong, such as when
    /// handing a list to another tool.
    pub async fn prefixed_titles_strict(&self) -> Result<Vec<String>, ToolsError> {
        let titles = self.prefixed_titles().await;
        if titles.len() != self.pages.len() {
            return Err(ToolsError::Tool(format!(
                "could not render namespace prefixes for {} of {} pages; is {} reachable?",
                self.pages.len() - titles.len(),
                self.pages.len(),
                self.site.wiki()
            )));
        }
        Ok(titles)
    }

    /// The list in the pre-0.2.0 format, with per-page metadata flattened to the top level.
    pub async fn as_json(&self) -> Value {
        let api = self.api_for_prefixed_titles().await;
        json!({
            "pages": self.pages
                .iter()
                .map(|page| page.to_legacy_json(api.as_ref()))
                .collect::<Vec<Value>>(),
            "site": self.site,
        })
    }

    /// An `Api` for rendering namespace prefixes, fetched only when some page needs one.
    /// Prefixes are cosmetic - a page's identity is its namespace id plus title - so a
    /// failure to reach the wiki omits `prefixed_title` rather than failing the operation.
    async fn api_for_prefixed_titles(&self) -> Option<Api> {
        if self.pages.iter().all(|page| page.title.namespace_id() == 0) {
            return None;
        }
        self.site.api().await.ok()
    }

    // MARK: Set operations
    //
    // All of these preserve the order of the left-hand list, so a ranking produced by a
    // tool (by page views, by link count, ...) survives being filtered against another list.

    /// Pages in either list. Metadata of pages in both is merged.
    pub async fn union(&self, other: &Self) -> Result<Self, ToolsError> {
        let other = self.aligned(other).await?;
        let mut index = Self::index(&other.pages);
        let mut pages: Vec<Page> = self
            .pages
            .iter()
            .map(|page| match index.remove(&page.key()) {
                Some(position) => page.merge(&other.pages[position]),
                None => page.clone(),
            })
            .collect();
        let mut remaining: Vec<usize> = index.into_values().collect();
        remaining.sort_unstable(); // Keep the right-hand list's own order
        pages.extend(
            remaining
                .into_iter()
                .map(|position| other.pages[position].clone()),
        );
        Ok(self.derive(pages, &other))
    }

    /// Pages in both lists, with their metadata merged.
    pub async fn intersection(&self, other: &Self) -> Result<Self, ToolsError> {
        let other = self.aligned(other).await?;
        let index = Self::index(&other.pages);
        let pages = self
            .pages
            .iter()
            .filter_map(|page| {
                index
                    .get(&page.key())
                    .map(|&position| page.merge(&other.pages[position]))
            })
            .collect();
        Ok(self.derive(pages, &other))
    }

    /// Pages in this list that are not in `other`.
    pub async fn difference(&self, other: &Self) -> Result<Self, ToolsError> {
        let other = self.aligned(other).await?;
        let index = Self::index(&other.pages);
        let pages = self
            .pages
            .iter()
            .filter(|page| !index.contains_key(&page.key()))
            .cloned()
            .collect();
        Ok(self.derive(pages, &other))
    }

    /// Pages in exactly one of the two lists.
    pub async fn symmetric_difference(&self, other: &Self) -> Result<Self, ToolsError> {
        let other = self.aligned(other).await?;
        let mine = Self::index(&self.pages);
        let theirs = Self::index(&other.pages);
        let mut pages: Vec<Page> = self
            .pages
            .iter()
            .filter(|page| !theirs.contains_key(&page.key()))
            .cloned()
            .collect();
        pages.extend(
            other
                .pages
                .iter()
                .filter(|page| !mine.contains_key(&page.key()))
                .cloned(),
        );
        Ok(self.derive(pages, &other))
    }

    fn index(pages: &[Page]) -> HashMap<String, usize> {
        pages
            .iter()
            .enumerate()
            .map(|(position, page)| (page.key(), position))
            .collect()
    }

    /// Returns `other` on this list's wiki, casting it if necessary.
    async fn aligned(&self, other: &Self) -> Result<Self, ToolsError> {
        if self.site == other.site {
            return Ok(other.clone());
        }
        other.to_wiki(self.site.wiki()).await
    }

    fn derive(&self, pages: Vec<Page>, other: &Self) -> Self {
        let mut sources = self.sources.clone();
        sources.extend(other.sources.iter().cloned());
        Self {
            site: self.site.clone(),
            pages,
            sources,
        }
    }

    // MARK: Casting between wikis

    /// Casts the list to another wiki via Wikidata sitelinks.
    ///
    /// Returns the pages that exist on `target_wiki`, and the pages that do not.
    /// The latter keeps this list's wiki, so it can be inspected or fed back into other
    /// operations. Cast pages record their previous title under `meta._cast`.
    pub async fn cast(&self, target_wiki: &str) -> Result<(Self, Self), ToolsError> {
        let target = Self::site_for_wiki(target_wiki)?;
        if target == self.site {
            return Ok((self.clone(), Self::new(self.site.clone())));
        }
        let source_api = self.site.api().await?;
        let titles: Vec<String> = self
            .pages
            .iter()
            .filter_map(|page| page.title.full_pretty(&source_api))
            .collect();
        let old_to_new = self.cast_map(target.wiki(), &titles).await?;
        let target_api = target.api().await?;

        let mut mapped = Self::new(target);
        let mut missing = Self::new(self.site.clone());
        mapped.sources = self.sources.clone();
        missing.sources = self.sources.clone();
        for page in &self.pages {
            let old_title = page.title.full_pretty(&source_api);
            match old_title.as_ref().and_then(|title| old_to_new.get(title)) {
                Some(new_title) => {
                    let mut new_page = page.clone();
                    // The target wiki's Api is required here: namespace names differ per wiki.
                    new_page.title = Title::new_from_full(new_title, &target_api);
                    new_page.set_meta_for(
                        CAST_META_KEY,
                        json!({"from": self.site.wiki(), "title": old_title}),
                    );
                    mapped.pages.push(new_page);
                }
                None => missing.pages.push(page.clone()),
            }
        }
        Ok((mapped, missing))
    }

    /// Casts the list to another wiki, dropping pages that do not exist there.
    pub async fn to_wiki(&self, target_wiki: &str) -> Result<Self, ToolsError> {
        Ok(self.cast(target_wiki).await?.0)
    }

    /// Maps titles on this wiki to titles on `target_wiki`.
    /// Uses `wd-infernal`, falling back to Wikidata's own API so that casting does not
    /// depend on a single tool being up.
    async fn cast_map(
        &self,
        target_wiki: &str,
        titles: &[String],
    ) -> Result<HashMap<String, String>, ToolsError> {
        match self.cast_map_via_infernal(target_wiki, titles).await {
            Ok(map) => Ok(map),
            Err(_) => {
                ToolsInterface::sitelinks_between_wikis(self.site.wiki(), target_wiki, titles).await
            }
        }
    }

    async fn cast_map_via_infernal(
        &self,
        target_wiki: &str,
        titles: &[String],
    ) -> Result<HashMap<String, String>, ToolsError> {
        let url = format!(
            "https://wd-infernal.toolforge.org/change_wiki/{source_wiki}/{target_wiki}",
            source_wiki = self.site.wiki()
        );
        let client = ToolsInterface::tokio_client()?;
        let map = client
            .post(url)
            .json(&json!(titles))
            .send()
            .await?
            .json()
            .await?;
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enwiki_list(titles: &[(&str, i64)]) -> PageList {
        let mut list = PageList::new(Site::from_wiki("enwiki").unwrap());
        for (title, namespace_id) in titles {
            list.pages.push(Page::new(Title::new(title, *namespace_id)));
        }
        list
    }

    #[tokio::test]
    async fn test_subset() {
        let pl1 = PageList::from_file("test_data/pagelist1.json").unwrap();
        let pl2 = PageList::from_file("test_data/pagelist2.json").unwrap();
        let pl3 = pl1.intersection(&pl2).await.unwrap();
        assert_eq!(pl3.pages.len(), 1);
    }

    #[tokio::test]
    async fn test_union() {
        let pl1 = PageList::from_file("test_data/pagelist1.json").unwrap();
        let pl2 = PageList::from_file("test_data/pagelist2.json").unwrap();
        let pl3 = pl1.union(&pl2).await.unwrap();
        assert_eq!(pl3.pages.len(), 12);
    }

    #[tokio::test]
    async fn test_difference() {
        let pl1 = PageList::from_file("test_data/pagelist1.json").unwrap();
        let pl2 = PageList::from_file("test_data/pagelist2.json").unwrap();
        let difference = pl1.difference(&pl2).await.unwrap();
        assert_eq!(difference.pages.len(), pl1.pages.len() - 1);
        let intersection = pl1.intersection(&pl2).await.unwrap();
        assert_eq!(difference.pages.len() + intersection.pages.len(), pl1.len());
    }

    #[tokio::test]
    async fn test_symmetric_difference() {
        let pl1 = PageList::from_file("test_data/pagelist1.json").unwrap();
        let pl2 = PageList::from_file("test_data/pagelist2.json").unwrap();
        let symmetric = pl1.symmetric_difference(&pl2).await.unwrap();
        let union = pl1.union(&pl2).await.unwrap();
        let intersection = pl1.intersection(&pl2).await.unwrap();
        assert_eq!(symmetric.len(), union.len() - intersection.len());
    }

    #[tokio::test]
    async fn test_set_operations_preserve_left_order() {
        let left = enwiki_list(&[("Zebra", 0), ("Apple", 0), ("Moon", 0)]);
        let right = enwiki_list(&[("Moon", 0), ("Apple", 0)]);
        let intersection = left.intersection(&right).await.unwrap();
        let titles: Vec<&str> = intersection
            .pages
            .iter()
            .map(|page| page.title.pretty())
            .collect();
        assert_eq!(titles, vec!["Apple", "Moon"]);
    }

    #[tokio::test]
    async fn test_union_is_deterministic() {
        let left = enwiki_list(&[("Apple", 0)]);
        let right = enwiki_list(&[("Beta", 0), ("Gamma", 0), ("Delta", 0), ("Epsilon", 0)]);
        let expected = vec!["Apple", "Beta", "Gamma", "Delta", "Epsilon"];
        // The right-hand pages used to be appended in HashMap iteration order.
        for _ in 0..10 {
            let union = left.union(&right).await.unwrap();
            let titles: Vec<&str> = union.pages.iter().map(|page| page.title.pretty()).collect();
            assert_eq!(titles, expected);
        }
    }

    #[test]
    fn test_metadata_is_namespaced_by_tool() {
        let from_petscan = json!({
            "site": {"wiki": "enwiki"},
            "pages": [{"title": "Earth", "namespace_id": 0, "counter": 1}],
        });
        let from_pageviews = json!({
            "site": {"wiki": "enwiki"},
            "pages": [{"title": "Earth", "namespace_id": 0, "counter": 99}],
        });
        let petscan = PageList::from_tool_json(&from_petscan, "petscan").unwrap();
        let pageviews = PageList::from_tool_json(&from_pageviews, "pageviews").unwrap();
        let merged = petscan.pages[0].merge(&pageviews.pages[0]);
        assert_eq!(merged.meta_for("petscan").unwrap()["counter"], json!(1));
        assert_eq!(merged.meta_for("pageviews").unwrap()["counter"], json!(99));
    }

    #[test]
    fn test_merge_within_one_tool_keeps_both_fields() {
        let mut first = Page::new(Title::new("Earth", 0));
        first.set_meta_for("xtools", json!({"revisions": 10}));
        let mut second = Page::new(Title::new("Earth", 0));
        second.set_meta_for("xtools", json!({"editors": 3}));
        let merged = first.merge(&second);
        let meta = merged.meta_for("xtools").unwrap();
        assert_eq!(meta["revisions"], json!(10));
        assert_eq!(meta["editors"], json!(3));
    }

    #[tokio::test]
    async fn test_jsonl_round_trip() {
        let mut list = enwiki_list(&[("Earth", 0), ("Category:Science", 14)]);
        list.pages[0].set_meta_for("petscan", json!({"counter": 7}));
        list.add_source(json!({"tool": "petscan"}));

        let mut buffer = Vec::new();
        list.to_writer(&mut buffer).await.unwrap();
        let text = String::from_utf8(buffer).unwrap();
        assert_eq!(text.lines().count(), 3); // Header plus two pages

        let read = PageList::from_reader(text.as_bytes(), None).unwrap();
        assert_eq!(read.site(), list.site());
        assert_eq!(read.pages(), list.pages());
        assert_eq!(read.sources(), list.sources());
    }

    #[tokio::test]
    async fn test_jsonl_survives_losing_the_header() {
        let list = enwiki_list(&[("Earth", 0)]);
        let mut buffer = Vec::new();
        list.to_writer(&mut buffer).await.unwrap();
        let text = String::from_utf8(buffer).unwrap();

        // What `jq 'select(...)'` would leave behind: pages only, no header.
        let without_header: String = text
            .lines()
            .skip(1)
            .map(|line| format!("{line}\n"))
            .collect();
        let read = PageList::from_reader(without_header.as_bytes(), None).unwrap();
        assert_eq!(read.site().wiki(), "enwiki");
        assert_eq!(read.len(), 1);
    }

    #[test]
    fn test_headerless_input_without_wiki_needs_an_override() {
        let text = "{\"title\":\"Earth\",\"namespace_id\":0}\n";
        assert!(PageList::from_reader(text.as_bytes(), None).is_err());
        let read = PageList::from_reader(text.as_bytes(), Some("enwiki")).unwrap();
        assert_eq!(read.site().wiki(), "enwiki");
        assert_eq!(read.len(), 1);
    }

    #[test]
    fn test_mixed_wikis_are_rejected() {
        let text = "{\"wiki\":\"enwiki\",\"title\":\"Earth\",\"namespace_id\":0}\n\
                    {\"wiki\":\"dewiki\",\"title\":\"Erde\",\"namespace_id\":0}\n";
        assert!(PageList::from_reader(text.as_bytes(), None).is_err());
        // ...but an explicit --wiki forces the issue.
        assert_eq!(
            PageList::from_reader(text.as_bytes(), Some("enwiki"))
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn test_reads_legacy_format() {
        let list = PageList::from_file("test_data/pagelist1.json").unwrap();
        assert_eq!(list.site().wiki(), "enwiki");
        assert_eq!(
            list.pages[0].meta_for(LEGACY_META_KEY).unwrap()["foo"],
            json!("bar")
        );
    }

    #[tokio::test]
    async fn test_cast() {
        let list = enwiki_list(&[("Biochemistry", 0), ("Magnus Manske", 0)]);
        let (mapped, missing) = list.cast("dewiki").await.unwrap();
        assert_eq!(mapped.site().wiki(), "dewiki");
        assert_eq!(mapped.pages.len(), 2);
        assert!(missing.is_empty());
        assert_eq!(mapped.pages[0].title.pretty(), "Biochemie");
        assert_eq!(mapped.pages[1].title.pretty(), "Magnus Manske");
        assert_eq!(
            mapped.pages[0].meta_for(CAST_META_KEY).unwrap()["title"],
            json!("Biochemistry")
        );
    }

    #[tokio::test]
    async fn test_cast_reports_missing_pages() {
        let list = enwiki_list(&[("Biochemistry", 0), ("Gay Nineties", 0)]);
        let (mapped, missing) = list.cast("dewiki").await.unwrap();
        assert_eq!(mapped.len() + missing.len(), 2);
        assert_eq!(missing.site().wiki(), "enwiki");
    }
}
