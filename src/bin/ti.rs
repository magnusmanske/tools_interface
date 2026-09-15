//! # Tools Interface - the binary
//! This binary is a command-line interface to the tools_interface library.
//! It allows you to run queries against various Wikimedia tools, and to combine their results.
//!
//! Use `ti help` to get the list of subommands,
//! and `ti help <subcommand>` to get help on a specific subcommand.
//!
//! Output is JSONL by default: one JSON object per line, so results can be piped
//! straight into another `ti` command, into `jq`, or into `wc -l`.
//! The first line is a header with the list's `site` and its provenance; every following
//! line is a page with a `title`, `prefixed_title`, `namespace_id`, and its `wiki`.
//! Tool-specific metadata sits under `meta`, keyed by the tool that produced it.
//!
//! Example: Run a PetScan query with a known PSID, and override two parameters:
//! ```shell
//! ti petscan --id 28348714 --params "foo=bar" "baz=123"
//! ```
//!
//! Example: Run Missing Topics on German Wikipedia for the article "Biologie", without template links:
//! ```shell
//! ti missing_topics --wiki dewiki --article Biologie --no_template_links
//! ```
//!
//! Lists compose, with `-` standing for stdin:
//! ```shell
//! # Pages a user created that are not in a PagePile
//! ti xtools_pages --wiki enwiki --user Magnus_Manske > mine.jsonl
//! ti pagepile --id 51805 > pile.jsonl
//! ti not-in mine.jsonl pile.jsonl
//!
//! # German articles missing an English counterpart, as a new PagePile
//! ti petscan --id 28348714 | ti cast --wiki enwiki --missing - | ti pagepile_create -
//! ```
//!
//! `jq` can filter the stream, because each page repeats its own `wiki`:
//! ```shell
//! # Just the titles
//! ti SOME_COMMAND --format titles
//! # Pages with more than 1000 views, as a list ti can still read
//! ti SOME_COMMAND | jq -c 'select(.title and .meta.pageviews.views > 1000)' | ti cat
//! ```
//! Guard such filters with `.title`, which only pages have, so they skip the header line.
//! Dropping the header is not fatal in any case: the wiki is recovered from the pages
//! themselves, and `ti cat` writes a fresh header. Only the `sources` provenance is lost.
//! Pass `--wiki` if the filter also removed the per-page `wiki` field.

use chrono::Utc;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use serde_json::{Value, json};
use std::io::Write;
use tools_interface::{
    AListBuildingTool, Completer, CompleterFilter, Duplicity, MissingTopics, PageList, PagePile,
    PetScan, QuickStatements, Site, Tool, ToolsError, grep::Grep, list_building::ListBuilding,
    search::WikiSearch, wiki_nearby::WikiNearby, xtools_pages::XtoolsPages,
};

/// Turns a tool's own JSON output into a `PageList`, then writes it.
/// `tool` names the metadata namespace the tool's per-page fields end up in.
async fn emit(tool: &str, json: &Value, params_all: &ArgMatches) -> Result<(), ToolsError> {
    let mut list = PageList::from_tool_json(json, tool)?;
    list.add_source(json!({"tool": tool, "at": Utc::now().to_rfc3339()}));
    write_page_list(&list, params_all).await
}

async fn write_page_list(list: &PageList, params_all: &ArgMatches) -> Result<(), ToolsError> {
    let format = params_all
        .get_one::<String>("format")
        .expect("--format missing");
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    match format.as_str() {
        "jsonl" => list.to_writer(&mut out).await?,
        "json" => writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&list.as_json().await)?
        )?,
        "titles" => {
            for title in list.prefixed_titles().await {
                writeln!(out, "{title}")?;
            }
        }
        other => return Err(ToolsError::Tool(format!("Unknown format: {other}"))),
    }
    Ok(())
}

fn site(wiki: &str) -> Result<Site, ToolsError> {
    Site::from_wiki(wiki).ok_or_else(|| ToolsError::Tool(format!("Unknown wiki '{wiki}'")))
}

/// The `--wiki` override for list-reading subcommands, forcing the wiki of the input.
fn wiki_override(params: &ArgMatches) -> Option<&str> {
    params.get_one::<String>("wiki").map(String::as_str)
}

fn subcommand<'a>(params_all: &'a ArgMatches, name: &str) -> &'a ArgMatches {
    params_all
        .subcommand_matches(name)
        .expect("No subcommand matches found")
}

async fn alistbuildingtool(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "alistbuildingtool");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let qid = params
        .get_one::<String>("item")
        .expect("--item missing")
        .to_ascii_uppercase();
    let mut tool = AListBuildingTool::new(site(wiki)?, &qid);
    tool.run().await?;
    emit("a_list_building_tool", &tool.as_json().await, params_all).await
}

async fn listbuilding(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "listbuilding");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let title = params.get_one::<String>("title").expect("--title missing");
    let mut tool = ListBuilding::new(site(wiki)?, title);
    tool.run().await?;
    emit("list_building", &tool.as_json().await, params_all).await
}

async fn wikinearby(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "wikinearby");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let title = params.get_one::<String>("title");
    let lat = params.get_one::<f64>("lat");
    let lon = params.get_one::<f64>("lon");
    let offset = params.get_one::<usize>("offset");
    let site = site(wiki)?;
    let mut tool = match (title, lat, lon) {
        (Some(title), _, _) => WikiNearby::new_from_page(site, title),
        (None, Some(lat), Some(lon)) => WikiNearby::new_from_coordinates(site, *lat, *lon),
        _ => {
            return Err(ToolsError::Tool(
                "either --title or both --lat and --lon are required".into(),
            ));
        }
    };
    if let Some(offset) = offset {
        tool.set_offset(*offset);
    }
    tool.run().await?;
    emit("wiki_nearby", &tool.as_json().await, params_all).await
}

async fn xtools_pages(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "xtools_pages");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let user = params.get_one::<String>("user").expect("--user missing");
    let namespace_id = params.get_one::<u32>("ns").expect("--ns missing"); // Has default value 0
    let mut tool = XtoolsPages::new(site(wiki)?, user).with_namespace_id(*namespace_id);
    tool.run().await?;
    emit("xtools_pages", &tool.as_json().await, params_all).await
}

async fn completer(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "completer");
    let from = params.get_one::<String>("from").expect("--from missing");
    let to = params.get_one::<String>("to").expect("--to missing");
    let psid = params.get_one::<String>("psid");
    let template = params.get_one::<String>("template");
    let category = params.get_one::<String>("category");
    let depth = params.get_one::<u32>("depth").expect("--depth missing");

    let mut tool = Completer::new(from, to);
    if let Some(psid) = psid {
        tool = tool.filter(CompleterFilter::PetScan {
            psid: psid.to_string(),
        });
    }
    if let Some(template) = template {
        tool = tool.filter(CompleterFilter::Template {
            template: template.to_string(),
        });
    }
    if let Some(category) = category {
        tool = tool.filter(CompleterFilter::Category {
            category: category.to_string(),
            depth: *depth,
        });
    }
    tool.run().await?;
    emit("completer", &tool.as_json().await, params_all).await
}

async fn duplicity(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "duplicity");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let mut tool = Duplicity::new(site(wiki)?);
    tool.run().await?;
    emit("duplicity", &tool.as_json().await, params_all).await
}

async fn search(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "search");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let query = params.get_one::<String>("query").expect("--query missing");
    let mut tool = WikiSearch::new(site(wiki)?, query);
    tool.run().await?;
    emit("search", &tool.as_json().await, params_all).await
}

async fn pagepile(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "pagepile");
    let id = params.get_one::<u32>("id").expect("--id missing");
    let mut tool = PagePile::new(*id);
    tool.run().await?;
    let json = tool
        .as_json()
        .await
        .ok_or_else(|| ToolsError::Tool(format!("PagePile {id} has no usable wiki")))?;
    emit("pagepile", &json, params_all).await
}

async fn petscan(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "petscan");
    let id = params.get_one::<u32>("id").expect("--id missing");
    let override_params = params
        .get_many::<String>("params")
        .unwrap_or_default()
        .collect::<Vec<_>>();
    let mut tool = PetScan::new(*id);
    for p in override_params {
        let (key, value) = p.split_once('=').ok_or_else(|| {
            ToolsError::Tool(format!("override parameter '{p}' is not 'key=value'"))
        })?;
        if key == "format" {
            eprintln!("Ignoring format override");
            continue;
        }
        tool.parameters_mut().retain(|(k, _)| k != key); // Remove old value, if any
        tool.parameters_mut()
            .push((key.to_string(), value.to_string())); // Add new value
    }
    tool.run().await?;
    emit("petscan", &tool.as_json().await, params_all).await
}

async fn missing_topics(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "missing_topics");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let article = params.get_one::<String>("article");
    let category = params.get_one::<String>("category");
    let depth = *params.get_one::<u32>("depth").expect("--depth missing");
    let no_template_links = params
        .get_one::<bool>("no_template_links")
        .copied()
        .unwrap_or_default();

    let mut tool = MissingTopics::new(site(wiki)?).no_template_links(no_template_links);
    if let Some(article) = article {
        tool = tool.with_article(article);
    }
    if let Some(category) = category {
        tool = tool.with_category(category, depth);
    }
    tool.run().await?;
    emit("missing_topics", &tool.as_json().await, params_all).await
}

async fn grep(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "grep");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let pattern = params
        .get_one::<String>("pattern")
        .expect("--pattern missing");
    let namespace_id = params.get_one::<usize>("ns").expect("--ns missing");
    let mut tool = Grep::new(site(wiki)?, pattern).with_namespace(*namespace_id);
    tool.run().await?;
    emit("grep", &tool.as_json().await, params_all).await
}

/// Combines two or more lists, left to right. Lists on other wikis are cast to the
/// wiki of the first one. The order of the first list is preserved, so a ranking
/// produced by a tool survives being combined with another list.
async fn set_operation(params_all: &ArgMatches, name: &str) -> Result<(), ToolsError> {
    let params = subcommand(params_all, name);
    let wiki = wiki_override(params);
    let files: Vec<&String> = params
        .get_many::<String>("files")
        .expect("two or more files required")
        .collect();
    if files.iter().filter(|file| *file == &"-").count() > 1 {
        return Err(ToolsError::Tool("stdin can only be read once".into()));
    }
    let mut result = PageList::from_path(files[0], wiki)?;
    for file in &files[1..] {
        let other = PageList::from_path(file, wiki)?;
        result = match name {
            "union" => result.union(&other).await?,
            "intersection" => result.intersection(&other).await?,
            "not-in" => result.difference(&other).await?,
            "sym-diff" => result.symmetric_difference(&other).await?,
            other => return Err(ToolsError::Tool(format!("Unknown operation {other}"))),
        };
    }
    result.add_source(json!({"operation": name, "at": Utc::now().to_rfc3339()}));
    write_page_list(&result, params_all).await
}

/// Moves a list to another wiki via Wikidata sitelinks.
async fn cast(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "cast");
    let target_wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let file = params.get_one::<String>("file").expect("file missing");
    let list = PageList::from_path(file, params.get_one::<String>("from").map(String::as_str))?;
    let (mapped, missing) = list.cast(target_wiki).await?;
    let mut result = if params.get_flag("missing") {
        missing
    } else {
        mapped
    };
    result.add_source(json!({
        "operation": "cast",
        "to": target_wiki,
        "missing": params.get_flag("missing"),
        "at": Utc::now().to_rfc3339(),
    }));
    write_page_list(&result, params_all).await
}

/// Reads a list and writes it out again. Converts between formats, validates input,
/// and re-attaches a header to a list that a `jq` filter has stripped it from.
async fn cat(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "cat");
    let file = params.get_one::<String>("file").expect("file missing");
    let list = PageList::from_path(file, wiki_override(params))?;
    write_page_list(&list, params_all).await
}

/// Creates a new PagePile from a list, and prints its ID.
async fn pagepile_create(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "pagepile_create");
    let file = params.get_one::<String>("file").expect("file missing");
    let list = PageList::from_path(file, wiki_override(params))?;
    if list.is_empty() {
        return Err(ToolsError::Tool(
            "refusing to create an empty PagePile".into(),
        ));
    }
    let id = PagePile::create(list.site(), &list.prefixed_titles_strict().await?).await?;
    println!(
        "{}",
        json!({
            "pagepile_id": id,
            "wiki": list.site().wiki(),
            "pages": list.len(),
            "url": format!("https://pagepile.toolforge.org/api.php?id={id}&action=get_data&doit"),
        })
    );
    Ok(())
}

/// Starts a QuickStatements batch from a list of Wikidata items.
async fn quickstatements(params_all: &ArgMatches) -> Result<(), ToolsError> {
    let params = subcommand(params_all, "quickstatements");
    let file = params.get_one::<String>("file").expect("file missing");
    let command = params
        .get_one::<String>("command")
        .expect("--command missing")
        .replace("\\t", "\t"); // So a tab-separated template survives the shell
    let list = PageList::from_path(file, wiki_override(params))?;

    if params.get_flag("dry_run") {
        let mut batch = QuickStatements::new("", "");
        let count = batch.add_commands_for_pages(&list, &command)?;
        eprintln!("{count} commands, not submitted (--dry_run)");
        print!("{}", batch.commands());
        return Ok(());
    }

    let user = params
        .get_one::<String>("user")
        .ok_or_else(|| ToolsError::Tool("--user is required unless --dry_run is given".into()))?;
    let token = params
        .get_one::<String>("token")
        .ok_or_else(|| ToolsError::Tool("--token is required unless --dry_run is given".into()))?;
    let mut batch = QuickStatements::new(user, token);
    if let Some(name) = params.get_one::<String>("batch_name") {
        batch = batch.batch_name(name);
    }
    let count = batch.add_commands_for_pages(&list, &command)?;
    batch.run().await?;
    println!(
        "{}",
        json!({"batch_id": batch.batch_id(), "commands": count})
    );
    Ok(())
}

/// The `--wiki` argument shared by the subcommands that read lists.
fn wiki_override_arg() -> Arg {
    Arg::new("wiki")
        .long("wiki")
        .help("Force the wiki of the input list (eg enwiki), overriding its header")
        .required(false)
}

fn list_input_arg() -> Arg {
    Arg::new("file")
        .help("A list file, or - for stdin")
        .default_value("-")
        .index(1)
}

fn set_operation_command(name: &'static str, about: &'static str) -> Command {
    Command::new(name)
        .about(about)
        .arg(
            Arg::new("files")
                .help("Two or more list files; - for stdin")
                .num_args(2..)
                .required(true)
                .index(1),
        )
        .arg(wiki_override_arg())
}

fn get_arg_matches() -> ArgMatches {
    Command::new("Tools Interface")
        .author("Magnus Manske <magnusmanske@googlemail.com>")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Runs queries against various Wikimedia tools, and combines their results")
        .arg(
            Arg::new("format")
                .default_value("jsonl")
                .long("format")
                .global(true) // Accepted before or after the subcommand
                .help("Output format: jsonl (default), json (pre-0.2.0 layout), or titles"),
        )
        .subcommands([
            Command::new("alistbuildingtool")
                .about("Retrieves pages from A List Building Tool")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("Wiki (eg enwiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("item")
                        .long("item")
                        .help("A Wikidatata item (eg Q42)")
                        .required(true),
                ),
            Command::new("listbuilding")
                .about("Retrieves pages from the List Building tool")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("Wiki (eg enwiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("title")
                        .long("title")
                        .help("A page on the given wiki")
                        .required(true),
                ),
            Command::new("completer")
                .about("Retrieves potential pages from Completer")
                .arg(
                    Arg::new("from")
                        .long("from")
                        .help("Source wikpedia language")
                        .required(true),
                )
                .arg(
                    Arg::new("to")
                        .long("to")
                        .help("Target wikpedia language")
                        .required(true),
                )
                .arg(
                    Arg::new("psid")
                        .long("psid")
                        .help("PetScan ID (optional)")
                        .required(false),
                )
                .arg(
                    Arg::new("template")
                        .long("template")
                        .help("Template (optional)")
                        .required(false),
                )
                .arg(
                    Arg::new("category")
                        .long("category")
                        .help("Category (optional)")
                        .required(false),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .help("Category depth (optional)")
                        .value_parser(value_parser!(u32))
                        .default_value("0")
                        .required(false),
                ),
            Command::new("duplicity")
                .about("Retrieves pages from Duplicity")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("Wiki (eg enwiki)")
                        .required(true),
                ),
            Command::new("pagepile")
                .about("Retrieves pages from PagePile")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_parser(value_parser!(u32))
                        .help("PagePile ID")
                        .required(true),
                ),
            Command::new("petscan")
                .about("Retrieves pages from PetScan")
                .arg(
                    Arg::new("id")
                        .long("id")
                        .value_parser(value_parser!(u32))
                        .help("PetScan ID")
                        .required(true),
                )
                .arg(
                    Arg::new("params")
                        .long("params")
                        .num_args(0..)
                        .help("Override parameters (optional; \"key=value\", multiple allowed)")
                        .required(false),
                ),
            Command::new("missing_topics")
                .about("Retrieves pages from Missing Topics")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("The wiki (eg enwiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("category")
                        .long("category")
                        .help("A category (optional)")
                        .required(false),
                )
                .arg(
                    Arg::new("article")
                        .long("article")
                        .help("An article (optional)")
                        .required(false),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .help("Category depth (optional)")
                        .value_parser(value_parser!(u32))
                        .default_value("0")
                        .required(false),
                )
                .arg(
                    Arg::new("no_template_links")
                        .long("no_template_links")
                        .help("No template links (optional)")
                        .action(ArgAction::SetTrue),
                ),
            Command::new("wikinearby")
                .about("Retrieves pages from WikiNearby")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("Wiki (eg enwiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("title")
                        .long("title")
                        .help("Page title")
                        .required(false),
                )
                .arg(
                    Arg::new("lat")
                        .long("lat")
                        .help("Latitude (requires --lon)")
                        .value_parser(value_parser!(f64))
                        .required(false),
                )
                .arg(
                    Arg::new("lon")
                        .long("lon")
                        .help("Longitude (requires --lat)")
                        .value_parser(value_parser!(f64))
                        .required(false),
                )
                .arg(
                    Arg::new("offset")
                        .long("offset")
                        .help("query offset (default:0)")
                        .value_parser(value_parser!(usize))
                        .required(false),
                ),
            Command::new("xtools_pages")
                .about("Retrieves pages from Xtools pages (created by a user)")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("Wiki (eg enwiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("user")
                        .long("user")
                        .help("Username")
                        .required(true),
                )
                .arg(
                    Arg::new("ns")
                        .long("namespace")
                        .help("Namespace ID")
                        .default_value("0")
                        .value_parser(value_parser!(u32))
                        .required(false),
                ),
            Command::new("search")
                .about("Performs a search on a wiki")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("Wiki (eg enwiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("query")
                        .long("query")
                        .help("Search query")
                        .required(true),
                ),
            Command::new("grep")
                .about("Queries the grep tool to search for page titles with a regular expression")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("Wiki (eg enwiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("pattern")
                        .long("pattern")
                        .help("RegExp pattern")
                        .required(true),
                )
                .arg(
                    Arg::new("ns")
                        .long("namespace")
                        .help("Namespace ID")
                        .default_value("0")
                        .value_parser(value_parser!(usize))
                        .required(false),
                ),
            set_operation_command(
                "union",
                "Pages in any of the lists. Merges metadata for pages in more than one",
            )
            .alias("merge"),
            set_operation_command(
                "intersection",
                "Pages in all of the lists. Merges metadata for duplicate pages",
            )
            .alias("subset"),
            set_operation_command(
                "not-in",
                "Pages in the first list that are in none of the others",
            )
            .alias("difference"),
            set_operation_command("sym-diff", "Pages in exactly one of two lists"),
            Command::new("cast")
                .about("Moves a list to another wiki, via Wikidata sitelinks")
                .arg(
                    Arg::new("wiki")
                        .long("wiki")
                        .help("The target wiki (eg dewiki)")
                        .required(true),
                )
                .arg(
                    Arg::new("from")
                        .long("from")
                        .help("Force the wiki of the input list, overriding its header")
                        .required(false),
                )
                .arg(
                    Arg::new("missing")
                        .long("missing")
                        .help("Output the pages that do NOT exist on the target wiki instead")
                        .action(ArgAction::SetTrue),
                )
                .arg(list_input_arg()),
            Command::new("cat")
                .about("Reads a list and writes it out again, converting between --format values")
                .arg(list_input_arg())
                .arg(wiki_override_arg()),
            Command::new("pagepile_create")
                .about("Creates a new PagePile from a list, and prints its ID")
                .arg(list_input_arg())
                .arg(wiki_override_arg()),
            Command::new("quickstatements")
                .about("Starts a QuickStatements batch from a list of Wikidata items")
                .arg(
                    Arg::new("command")
                        .long("command")
                        .help("A V1 command template; {item} and {title} are substituted, \\t becomes a tab")
                        .required(true),
                )
                .arg(
                    Arg::new("user")
                        .long("user")
                        .help("Your Wikimedia user name")
                        .required(false),
                )
                .arg(
                    Arg::new("token")
                        .long("token")
                        .help("Your QuickStatements token")
                        .required(false),
                )
                .arg(
                    Arg::new("batch_name")
                        .long("batch_name")
                        .help("A name for the batch (optional)")
                        .required(false),
                )
                .arg(
                    Arg::new("dry_run")
                        .long("dry_run")
                        .help("Print the commands instead of submitting them")
                        .action(ArgAction::SetTrue),
                )
                .arg(list_input_arg())
                .arg(wiki_override_arg()),
        ])
        .get_matches()
}

async fn run(m: &ArgMatches) -> Result<(), ToolsError> {
    match m.subcommand_name() {
        Some("alistbuildingtool") => alistbuildingtool(m).await,
        Some("cast") => cast(m).await,
        Some("completer") => completer(m).await,
        Some("duplicity") => duplicity(m).await,
        Some("grep") => grep(m).await,
        Some("listbuilding") => listbuilding(m).await,
        Some("missing_topics") => missing_topics(m).await,
        Some("pagepile") => pagepile(m).await,
        Some("pagepile_create") => pagepile_create(m).await,
        Some("petscan") => petscan(m).await,
        Some("quickstatements") => quickstatements(m).await,
        Some("search") => search(m).await,
        Some("cat") => cat(m).await,
        Some("wikinearby") => wikinearby(m).await,
        Some("xtools_pages") => xtools_pages(m).await,
        Some(operation @ ("union" | "intersection" | "not-in" | "sym-diff")) => {
            set_operation(m, operation).await
        }
        Some(other) => Err(ToolsError::Tool(format!(
            "Unknown subcommand given: {other}"
        ))),
        None => Err(ToolsError::Tool("No subcommand given".to_string())),
    }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let m = get_arg_matches();
    match run(&m).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
