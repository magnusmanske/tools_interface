pub static TOOLS_INTERFACE_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);


pub mod error;
pub mod persondata_template;
pub mod pagepile;
pub mod petscan;

pub use error::ToolsError;
pub use persondata_template::*;
pub use pagepile::*;
pub use petscan::*;
