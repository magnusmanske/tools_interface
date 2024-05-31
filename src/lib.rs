//! # Tools Interface
//!
//! This rust crate implements structs to easily interface with several Wikipedia/Wikidata/Wikimedia tools and APIs.
//!
//! ## Supported tools
//!
//! - [Completer](https://completer.toolforge.org/)
//! - [PagePile](https://pagepile.toolforge.org/) (read only)
//! - [Pageviews API](https://wikitech.wikimedia.org/wiki/Analytics/AQS/Pageviews)
//! - [Persondata Template](https://persondata.toolforge.org/vorlagen/)
//! - [PetScan](https://petscan.wmflabs.org/)
//! - [Missing Topics](https://missingtopics.toolforge.org/)
//! - [Quarry](https://quarry.wmcloud.org/) (existing results only)
//! - [QuickStatements](https://quickstatements.toolforge.org/) (start batches)
//! - [SparqlRC](https://wikidata-todo.toolforge.org/sparql_rc.php)
//!
//! If you would like to see other tools supported, add a request to the [Issue tracker](https://github.com/magnusmanske/tools_interface/issues).

pub mod completer;
pub mod error;
pub mod missing_topics;
pub mod pagepile;
pub mod pageviews;
pub mod persondata_template;
pub mod petscan;
pub mod quarry;
pub mod quickstatements;
pub mod site;
pub mod sparql_rc;
pub mod tools_interface;

pub use completer::{Completer, CompleterFilter};
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
pub use tools_interface::ToolsInterface;

/*
TODO
- https://a-list-bulding-tool.toolforge.org ?
- WD-FIST
- https://xtools.wmcloud.org/pages (parse wikitext output)
- https://ws-search.toolforge.org/ (needs HTML scraping?)
- https://wp-trending.toolforge.org/
- https://wikinearby.toolforge.org/ (via its API)
- https://wikidata-todo.toolforge.org/user_edits.php
- https://wikidata-todo.toolforge.org/wd_edit_stats.php
- https://wikidata-todo.toolforge.org/wdq_image_feed.php
- https://fist.toolforge.org/wd4wp/#/
- https://wikidata-todo.toolforge.org/duplicity/#/
- https://whattodo.toolforge.org
- https://checkwiki.toolforge.org/checkwiki.cgi
- https://cil2.toolforge.org/
- https://grep.toolforge.org/
- https://nppbrowser.toolforge.org/
- https://searchsbl.toolforge.org/
- https://item-quality-evaluator.toolforge.org (to add scores)
- topicmatcher
*/
