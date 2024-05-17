use std::{error::Error, fmt::{self, Display, Formatter}};

#[derive(Debug)]
pub enum ToolsError {
    Reqwest(reqwest::Error),
    Csv(csv::Error),
}

impl Display for ToolsError {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ToolsError::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            ToolsError::Csv(e) => write!(f, "CSV error: {}", e),
        }
    }
}

impl Error for ToolsError {
}

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
