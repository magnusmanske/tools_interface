//! # Pageviews
//! This implements a simple interface to the Wikimedia Pageviews API.
//! More information can be found [here](https://wikitech.wikimedia.org/wiki/Analytics/AQS/Pageviews).
//! Currently, only single-page views are supported.
//! Aggregate and top views are not yet implemented.
//!
//! ## Features
//! Views for multiple pages, on multiple projects, can be retrieved concurrently for a single time span.
//!
//! ## Example
//! ```rust
//! let pv = Pageviews::new(
//!     PageviewsGranularity::Monthly, // Get monthly views
//!     PageviewsAccess::All, // Get all-access views
//!     PageviewsAgent::All, // Get views from all agents
//! );
//!
//! // Prepre a `(String,String)` vector of project-page pairs.
//! let project_pages = [
//!     ("de.wikipedia", "Barack Obama"),
//!     ("de.wikipedia", "Trude Herr"),
//! ].into_iter().map(|(a, b)| (a.into(), b.into())).collect();
//!
//! // Get the pageviews for these pages for every month of 2016.
//! let results = pv.get_multiple_articles(
//!     &project_pages,
//!     &Pageviews::month_start(2016, 1).unwrap(),
//!     &Pageviews::month_end(2016, 12).unwrap(),
//!     5,
//! ).await.unwrap();
//!
//! // Count all views of all pages.
//! let overall_views: u64 = results.iter().map(|r| r.total_views()).sum();
//! ```

// TODO Use `Tool` trait!

use chrono::{Duration, NaiveDate};
use futures::prelude::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum PageviewsAccess {
    #[serde(rename = "all-access")]
    All,
    #[serde(rename = "desktop")]
    Desktop,
    #[serde(rename = "mobile-app")]
    MobileApp,
    #[serde(rename = "mobile-web")]
    MobileWeb,
}

impl PageviewsAccess {
    pub fn as_str(&self) -> &str {
        match self {
            Self::All => "all-access",
            Self::Desktop => "desktop",
            Self::MobileApp => "mobile-app",
            Self::MobileWeb => "mobile-web",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum PageviewsAgent {
    #[serde(rename = "all-agents")]
    All,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "spider")]
    Spider,
    #[serde(rename = "automated")]
    Automated,
}

impl PageviewsAgent {
    pub fn as_str(&self) -> &str {
        match self {
            Self::All => "all-agents",
            Self::User => "user",
            Self::Spider => "spider",
            Self::Automated => "automated",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub enum PageviewsGranularity {
    #[serde(rename = "hourly")]
    Hourly,
    #[serde(rename = "daily")]
    Daily,
    #[serde(rename = "monthly")]
    Monthly,
}

impl PageviewsGranularity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Monthly => "monthly",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct PageviewsTimestamp {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
}

impl From<&str> for PageviewsTimestamp {
    fn from(item: &str) -> Self {
        Self {
            year: item[0..4].parse().unwrap(),
            month: item[4..6].parse().unwrap(),
            day: item[6..8].parse().unwrap(),
            hour: item[8..10].parse().unwrap(),
        }
    }
}

impl Into<String> for PageviewsTimestamp {
    fn into(self) -> String {
        format!(
            "{:04}{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageviewsParams {
    pub timestamp: PageviewsTimestamp,
    pub views: u64,
}

impl PageviewsParams {
    fn from_json(item: &Value) -> Option<Self> {
        let ts = item.get("timestamp")?.as_str()?;
        Some(Self {
            timestamp: ts.into(),
            views: item.get("views")?.as_u64()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PageviewsResult {
    pub project: String,
    pub article: String,
    pub granularity: PageviewsGranularity,
    pub access: PageviewsAccess,
    pub agent: PageviewsAgent,
    pub entries: Vec<PageviewsParams>,
}

impl PageviewsResult {
    pub fn total_views(&self) -> u64 {
        self.entries.iter().map(|r| r.views).sum::<u64>()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[derive(Debug, PartialEq)]
pub struct Pageviews {
    granularity: PageviewsGranularity,
    access: PageviewsAccess,
    agent: PageviewsAgent,
}

impl Pageviews {
    // Returns a `NaiveDate` representing the first day of the month.
    pub fn month_start(year: i32, month: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(year, month, 1)
    }

    // Returns a `NaiveDate` representing the last day of the month.
    pub fn month_end(year: i32, month: u32) -> Option<NaiveDate> {
        let mut last_day_of_month = NaiveDate::from_ymd_opt(year, month + 1, 1)
            .or(NaiveDate::from_ymd_opt(year + 1, 1, 1))?;
        last_day_of_month -= Duration::days(1);
        Some(last_day_of_month)
    }

    /// Create a new `Pageviews` object.
    pub fn new(
        granularity: PageviewsGranularity,
        access: PageviewsAccess,
        agent: PageviewsAgent,
    ) -> Self {
        Self {
            granularity,
            access,
            agent,
        }
    }

    #[cfg(feature = "tokio")]
    /// Get pageviews for a single page.
    /// The result page title will have underscores ("_") instead of spaces.
    /// This function will automatically retry if the Wikimedia API returns a 429 (throttling) status code.
    pub async fn get_per_article<S1: Into<String>, S2: Into<String>>(
        &self,
        page: S1,
        project: S2,
        start: &NaiveDate,
        end: &NaiveDate,
    ) -> Result<PageviewsResult, crate::ToolsError> {
        let project: String = project.into();
        let page: String = page.into().replace(" ", "_");
        let url = format!("https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article/{project}/{access}/{agent}/{page}/{granularity}/{start}/{end}",
            access=self.access.as_str(),
            agent=self.agent.as_str(),
            granularity=self.granularity.as_str(),
            start=start.format("%Y%m%d").to_string(),
            end=end.format("%Y%m%d").to_string(),
        );
        let client = crate::ToolsInterface::tokio_client()?;
        let json: Value;
        loop {
            let response = client.get(&url).send().await?;
            let status = response.status();
            if status == 429 {
                // Throttling
                let delay = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|s| s.to_str().ok())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5);
                tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                continue;
            }
            json = response.json().await?;
            break;
        }
        if json.get("status").is_some() {
            let message = match json.get("detail") {
                Some(detail) => match detail.as_str() {
                    Some(detail_str) => detail_str.to_string(),
                    None => detail.to_string(), // Not a string, fallback
                },
                None => json["status"].to_string(), // We know this exists, fallback
            };
            return Err(crate::ToolsError::Tool(message));
        }
        let items = json
            .get("items")
            .ok_or_else(|| crate::ToolsError::Json("No 'items' in Pageviews JSON".to_string()))?
            .as_array()
            .ok_or_else(|| {
                crate::ToolsError::Json("'items' is not an array in Pageviews JSON".to_string())
            })?;
        let ret = PageviewsResult {
            project: project,
            article: page.into(),
            granularity: self.granularity.to_owned(),
            access: self.access.to_owned(),
            agent: self.agent.to_owned(),
            entries: items
                .iter()
                .filter_map(|item| PageviewsParams::from_json(item))
                .collect(),
        };
        Ok(ret)
    }

    #[cfg(feature = "tokio")]
    /// Get pageviews for multiple pages.
    /// The page titles in the results will have underscores ("_") instead of spaces.
    /// Use a low `max_concurrent` value to avoid hitting the Wikimedia API rate limits.
    /// Failed requests will be silently ignored.
    pub async fn get_multiple_articles(
        &self,
        project_pages: &Vec<(String, String)>,
        start: &NaiveDate,
        end: &NaiveDate,
        max_concurrent: usize,
    ) -> Result<Vec<PageviewsResult>, crate::ToolsError> {
        let mut futures = Vec::new();
        for (project, page) in project_pages {
            let fut = self.get_per_article(page, project, start, end);
            futures.push(fut);
        }
        let stream = futures::stream::iter(futures).buffer_unordered(max_concurrent);
        let results = stream.collect::<Vec<_>>().await;
        Ok(results.into_iter().filter_map(|r| r.ok()).collect())
    }

    // TODO aggregate (all-projects)
    // TODO top
    // TODO top-per-country
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_last_of_month() {
        assert_eq!(Pageviews::month_end(2021, 1).unwrap().format("%Y-%m-%d").to_string(), "2021-01-31");
        assert_eq!(Pageviews::month_end(2021, 2).unwrap().format("%Y-%m-%d").to_string(), "2021-02-28");
        assert_eq!(Pageviews::month_end(2024, 2).unwrap().format("%Y-%m-%d").to_string(), "2024-02-29"); // Leap year
        assert_eq!(Pageviews::month_end(2021, 3).unwrap().format("%Y-%m-%d").to_string(), "2021-03-31");
        assert_eq!(Pageviews::month_end(2021, 4).unwrap().format("%Y-%m-%d").to_string(), "2021-04-30");
        assert_eq!(Pageviews::month_end(2021, 5).unwrap().format("%Y-%m-%d").to_string(), "2021-05-31");
        assert_eq!(Pageviews::month_end(2021, 6).unwrap().format("%Y-%m-%d").to_string(), "2021-06-30");
        assert_eq!(Pageviews::month_end(2021, 7).unwrap().format("%Y-%m-%d").to_string(), "2021-07-31");
        assert_eq!(Pageviews::month_end(2021, 8).unwrap().format("%Y-%m-%d").to_string(), "2021-08-31");
        assert_eq!(Pageviews::month_end(2021, 9).unwrap().format("%Y-%m-%d").to_string(), "2021-09-30");
        assert_eq!(Pageviews::month_end(2021, 10).unwrap().format("%Y-%m-%d").to_string(), "2021-10-31");
        assert_eq!(Pageviews::month_end(2021, 11).unwrap().format("%Y-%m-%d").to_string(), "2021-11-30");
        assert_eq!(Pageviews::month_end(2021, 12).unwrap().format("%Y-%m-%d").to_string(), "2021-12-31");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pageviews_get_per_article_monthly_async() {
        let pv = Pageviews::new(
            PageviewsGranularity::Monthly,
            PageviewsAccess::All,
            PageviewsAgent::All,
        );
        let result = pv
            .get_per_article(
                "Barack_Obama",
                "de.wikipedia",
                &Pageviews::month_start(2016, 1).unwrap(),
                &Pageviews::month_end(2016, 12).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.len(), 12);
        assert_eq!(result.total_views(), 1_550_502);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pageviews_get_per_article_daily_async() {
        let pv = Pageviews::new(
            PageviewsGranularity::Daily,
            PageviewsAccess::All,
            PageviewsAgent::All,
        );
        let result = pv
            .get_per_article(
                "Barack_Obama",
                "de.wikipedia",
                &Pageviews::month_start(2016, 1).unwrap(),
                &Pageviews::month_end(2016, 1).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.len(), 31);
        assert_eq!(result.total_views(), 112_458);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pageviews_get_per_article_bad_date_async() {
        let pv = Pageviews::new(
            PageviewsGranularity::Daily,
            PageviewsAccess::All,
            PageviewsAgent::All,
        );
        let result = pv
            .get_per_article(
                "Barack_Obama",
                "de.wikipedia",
                &Pageviews::month_start(1016, 1).unwrap(),
                &Pageviews::month_end(1016, 1).unwrap(),
            )
            .await;
        assert!(result.is_err());
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn test_pageviews_multiple_articles_async() {
        let pv = Pageviews::new(
            PageviewsGranularity::Monthly,
            PageviewsAccess::All,
            PageviewsAgent::All,
        );
        let project_pages = [
            ("de.wikipedia", "Barack Obama"),
            ("de.wikipedia", "Trude Herr"),
        ]
        .into_iter()
        .map(|(a, b)| (a.into(), b.into()))
        .collect();
        let results = pv
            .get_multiple_articles(
                &project_pages,
                &Pageviews::month_start(2016, 1).unwrap(),
                &Pageviews::month_end(2016, 12).unwrap(),
                5,
            )
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
        let overall_views: u64 = results.iter().map(|r| r.total_views()).sum();
        assert_eq!(overall_views, 1_670_723);
    }

    #[test]
    fn test_pageviews_timestamp() {
        let time_string = "2345123159";
        let ts: PageviewsTimestamp = time_string.into();
        let ts: String = ts.into();
        assert_eq!(ts, time_string);
    }
}
