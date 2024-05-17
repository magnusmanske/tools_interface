pub static TOOLS_INTERFACE_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);


pub mod error;
pub mod persondata_vorlage;

pub use error::ToolsError;
pub use persondata_vorlage::*;
