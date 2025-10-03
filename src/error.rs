/// # Error module
/// `ToolsError` is a wrapper around several error types that can be returned by the tools.
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use mediawiki::media_wiki_error::MediaWikiError;

#[derive(Debug)]
pub enum ToolsError {
    Tool(String),
    Reqwest(reqwest::Error),
    Csv(csv::Error),
    Json(String),
    SerdeJson(serde_json::Error),
    MediaWiki(MediaWikiError),
    Io(std::io::Error),
}

impl Display for ToolsError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ToolsError::Tool(e) => write!(f, "Tool error: {}", e),
            ToolsError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            ToolsError::Csv(e) => write!(f, "CSV error: {}", e),
            ToolsError::Json(e) => write!(f, "JSON error: {}", e),
            ToolsError::SerdeJson(e) => write!(f, "Serde JSON error: {}", e),
            ToolsError::MediaWiki(e) => write!(f, "MediaWiki error: {}", e),
            ToolsError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl Error for ToolsError {}

impl From<reqwest::Error> for ToolsError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl From<csv::Error> for ToolsError {
    fn from(e: csv::Error) -> Self {
        Self::Csv(e)
    }
}

impl From<serde_json::Error> for ToolsError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerdeJson(e)
    }
}

impl From<MediaWikiError> for ToolsError {
    fn from(e: MediaWikiError) -> Self {
        Self::MediaWiki(e)
    }
}

impl From<std::io::Error> for ToolsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
