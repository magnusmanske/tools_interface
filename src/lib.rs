//! # Tools Interface
//!
//! This rust crate implements structs to easily interface with several Wikipedia/Wikidata/Wikimedia tools and APIs.
//!
//! ## Supported tools
//!
//! - [A List Building Tool](https://a-list-bulding-tool.toolforge.org/)
//! - [Completer](https://completer.toolforge.org/)
//! - [Duplicity](https://wikidata-todo.toolforge.org/duplicity/)
//! - [List Building](https://list-building.toolforge.org)
//! - [PagePile](https://pagepile.toolforge.org/) (read only)
//! - [Pageviews API](https://wikitech.wikimedia.org/wiki/Analytics/AQS/Pageviews)
//! - [Persondata Template](https://persondata.toolforge.org/vorlagen/)
//! - [PetScan](https://petscan.wmflabs.org/)
//! - [Missing Topics](https://missingtopics.toolforge.org/)
//! - [Quarry](https://quarry.wmcloud.org/) (retrieve existing results only)
//! - [QuickStatements](https://quickstatements.toolforge.org/) (create and start batches)
//! - [SparqlRC](https://wikidata-todo.toolforge.org/sparql_rc.php)
//! - [WikiNearby](https://wikinearby.toolforge.org/)
//! - [XTools pages](https://xtools.wmcloud.org/pages)
//!
//! If you would like to see other tools supported, add a request to the [Issue tracker](https://github.com/magnusmanske/tools_interface/issues).

pub mod a_list_building_tool;
pub mod completer;
pub mod duplicity;
pub mod error;
pub mod list_building;
pub mod missing_topics;
pub mod pagepile;
pub mod pageviews;
pub mod persondata_template;
pub mod petscan;
pub mod quarry;
pub mod quickstatements;
pub mod site;
pub mod sparql_rc;
pub mod tool;
pub mod tools_interface;
pub mod wiki_nearby;
pub mod xtools_pages;

pub use a_list_building_tool::AListBuildingTool;
pub use completer::{Completer, CompleterFilter};
pub use duplicity::Duplicity;
pub use error::ToolsError;
pub use missing_topics::MissingTopics;
pub use pagepile::PagePile;
pub use pageviews::*;
pub use persondata_template::*;
pub use petscan::*;
pub use quarry::Quarry;
pub use quickstatements::QuickStatements;
pub use site::Site;
pub use sparql_rc::{EntityEdit, EntityEditor, SparqlRC};
pub use tool::Tool;
pub use tools_interface::ToolsInterface;

/*
TEST:
cargo test --lib --tests --bins

TODO
- WD-FIST
- https://ws-search.toolforge.org/ (needs HTML scraping?)
- https://wp-trending.toolforge.org/
- https://wikidata-todo.toolforge.org/user_edits.php
- https://wikidata-todo.toolforge.org/wd_edit_stats.php
- https://wikidata-todo.toolforge.org/wdq_image_feed.php
- https://fist.toolforge.org/wd4wp/#/
- https://whattodo.toolforge.org
- https://checkwiki.toolforge.org/checkwiki.cgi
- https://grep.toolforge.org/ [DEFUNCT?]
- https://nppbrowser.toolforge.org/
- https://searchsbl.toolforge.org/
- https://item-quality-evaluator.toolforge.org (to add scores)
- topicmatcher
*/
