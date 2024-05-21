pub mod error;
pub mod pagepile;
pub mod pageviews;
pub mod persondata_template;
pub mod petscan;
pub mod quarry;
pub mod quickstatements;
pub mod tools_interface;

pub use error::ToolsError;
pub use pagepile::*;
pub use pageviews::*;
pub use persondata_template::*;
pub use petscan::*;
pub use quarry::*;
pub use quickstatements::*;
pub use tools_interface::*;

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
- https://wikidata-todo.toolforge.org/sparql_rc.php
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
