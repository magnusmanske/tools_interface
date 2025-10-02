/// # List building
/// Module for interacting with the [list building](https://list-building.toolforge.org) tool.
/// You can retrieve a list of pages on one wiki that relate to a Wiki page.
/// There are blocking and async methods available.
///
/// ## Example
/// ```ignore
/// let site = Site::from_wiki("enwiki").unwrap();
/// let title = "SARS-CoV-2";
/// let mut a = XtoolsPages::new(site, title);
/// a.run().await.unwrap();
/// a.results()
///     .iter()
///     .for_each(|result| {
///        println!("Page {} Item {} Description {}", result.title, result.qid, result.description);
///     });
/// ```
use crate::{Site, Tool, ToolsError};
use async_trait::async_trait;
use chrono::{NaiveDate, NaiveDateTime};

#[derive(Debug, Default, PartialEq)]
pub struct XtoolsPagesResult {
    pub title: String,
    pub namespace_id: u32,
    pub date: NaiveDateTime,
    pub original_size: u32,
    pub current_size: u32,
    pub assessment: String,
}

impl XtoolsPagesResult {
    fn from_tsv_row(row: &str) -> Option<Self> {
        let mut row = row.split("\t");
        let namespace_id = row.next()?.parse::<u32>().ok()?;
        let title = row.next()?;
        let date = row.next()?;
        let original_size = row.next()?.parse::<u32>().ok()?;
        let current_size = row.next()?.parse::<u32>().ok()?;
        let assessment = row.next()?;

        // This should have been the last column
        if row.next().is_some() {
            return None;
        }

        Some(Self {
            title: title.to_string(),
            namespace_id,
            date: NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M").ok()?,
            original_size,
            current_size,
            assessment: assessment.to_string(),
        })
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum DeletedPages {
    #[default]
    All,
    Live,
    Deleted,
}

impl DeletedPages {
    fn as_str(&self) -> &str {
        match self {
            DeletedPages::All => "all",
            DeletedPages::Live => "live",
            DeletedPages::Deleted => "deleted",
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum Redirects {
    None,
    #[default]
    All,
    OnlyRedirects,
}

impl Redirects {
    fn as_str(&self) -> &str {
        match self {
            Redirects::None => "noredirects",
            Redirects::All => "all",
            Redirects::OnlyRedirects => "onlyredirects",
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct XtoolsPages {
    site: Site,
    user: String,
    namespace_id: Option<u32>,
    redirects: Redirects,
    deleted_pages: DeletedPages,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    results: Vec<XtoolsPagesResult>,
}

impl XtoolsPages {
    pub fn new(site: Site, user: &str) -> Self {
        Self {
            site,
            user: user.to_string(),
            ..Default::default()
        }
    }

    pub fn with_namespace_id(mut self, namespace_id: u32) -> Self {
        self.namespace_id = Some(namespace_id);
        self
    }

    pub fn with_redirects(mut self, redirects: Redirects) -> Self {
        self.redirects = redirects;
        self
    }

    pub fn with_deleted_pages(mut self, deleted_pages: DeletedPages) -> Self {
        self.deleted_pages = deleted_pages;
        self
    }

    pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
        self.start_date = Some(start_date);
        self
    }

    pub fn with_end_date(mut self, end_date: NaiveDate) -> Self {
        self.end_date = Some(end_date);
        self
    }

    pub fn results(&self) -> &[XtoolsPagesResult] {
        &self.results
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    pub fn namespace_id(&self) -> Option<u32> {
        self.namespace_id
    }

    pub fn start_date(&self) -> Option<NaiveDate> {
        self.start_date
    }

    pub fn end_date(&self) -> Option<NaiveDate> {
        self.end_date
    }

    pub fn redirects(&self) -> &Redirects {
        &self.redirects
    }

    pub fn deleted_pages(&self) -> &DeletedPages {
        &self.deleted_pages
    }
}

#[async_trait]
impl Tool for XtoolsPages {
    fn get_url(&self) -> String {
        let url = format!(
            "https://xtools.wmcloud.org/pages/{server}/{user}/{namespace_id}/{redirects}/{deleted_pages}/{start_date}/{end_date}?format=tsv",
            server = self.site.webserver(),
            user = self.user,
            namespace_id = self.namespace_id.unwrap_or(0),
            redirects = self.redirects.as_str(),
            deleted_pages = self.deleted_pages.as_str(),
            start_date = self.start_date.unwrap_or_default(),
            end_date = self.end_date.unwrap_or_default(),
        );
        url
    }

    #[cfg(feature = "blocking")]
    /// Run the tool in a blocking manner.
    fn run_blocking(&mut self) -> Result<(), ToolsError> {
        let url = self.get_url();
        let client = crate::ToolsInterface::blocking_client()?;
        let text = client.get(&url).send()?.text()?;
        self.set_from_text(&text)
    }

    #[cfg(feature = "tokio")]
    /// Run the tool asynchronously.
    async fn run(&mut self) -> Result<(), ToolsError> {
        let url = self.get_url();
        let client = crate::ToolsInterface::tokio_client()?;
        let text = client.get(&url).send().await?.text().await?;
        self.set_from_text(&text)
    }

    fn set_from_text(&mut self, text: &str) -> Result<(), ToolsError> {
        self.results = text
            .split("\n")
            .skip(1)
            .filter_map(XtoolsPagesResult::from_tsv_row)
            .collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let site = Site::from_wiki("enwiki").unwrap();
        let user = "Magnus Manske";
        let end_date = NaiveDate::parse_from_str("2024-12-31", "%Y-%m-%d").unwrap();
        let tool = XtoolsPages::new(site.clone(), user).with_end_date(end_date);
        assert_eq!(tool.site(), &site);
        assert_eq!(tool.user(), user);
        assert_eq!(tool.end_date(), Some(end_date));
    }

    #[tokio::test]
    async fn test_xtools_run() {
        let site = Site::from_wiki("enwiki").unwrap();
        let user = "Magnus Manske";
        let end_date = NaiveDate::parse_from_str("2024-12-31", "%Y-%m-%d").unwrap();
        let mut tool = XtoolsPages::new(site.clone(), user)
            .with_deleted_pages(DeletedPages::All)
            .with_end_date(end_date);
        tool.run().await.unwrap();
        assert_eq!(tool.results().len(), 985);
    }
}
