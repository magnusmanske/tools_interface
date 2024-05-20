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
}
