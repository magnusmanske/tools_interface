//! # Tools Interface - the binary
//! This binary is a command-line interface to the tools_interface library.
//! It allows you to run queries against various Wikimedia tools.
//!
//! Use `ti help` to get the list of subommands,
//! and `ti help <subcommand>` to get help on a specific subcommand.
//!
//! Default output format is JSON, so you can pipe the output to `jq` for downstream processing.
//! Pages are listed in the `.pages` array, with each page having a `title`, a `prefixed_title`, and a `namespace_id`.
//! Each page can have additional fields, depending on the tool used.
//! The `.site` object contains the result site's wiki, language and project.
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
//! To convert the output to a more human-readable format, you can use `jq`:
//! ```shell
//! # First, pipe your output to a file:
//! ti SOME_COMMAND > test.json
//! # Assuming you just want the page titles:
//! jq -r '.pages[].prefixed_title' < test.json
//! # Assuming the output has additional `counter` fields:
//! jq -r '.pages[] | "\(.prefixed_title)\t\(.counter)"' < test.json
//! ```

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use serde_json::Value;
use tools_interface::{
    AListBuildingTool, Completer, CompleterFilter, Duplicity, MissingTopics, PagePile, PetScan,
    Site, Tool, grep::Grep, list_building::ListBuilding, page_list::PageList, search::WikiSearch,
    wiki_nearby::WikiNearby, xtools_pages::XtoolsPages,
};

fn write_json(j: &Value) {
    println!("{}", serde_json::to_string_pretty(&j).unwrap());
}

fn write_output(out: &Value, params_all: &ArgMatches) {
    let format = params_all
        .get_one::<String>("format")
        .expect("--format missing");
    match format.as_str() {
        "json" => write_json(out),
        _ => eprintln!("Unknown format: {format}"),
    }
}

async fn alistbuildingtool(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("alistbuildingtool")
        .expect("No subcommand matches found");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let qid = params
        .get_one::<String>("item")
        .expect("--item missing")
        .to_ascii_uppercase();
    let mut tool = AListBuildingTool::new(Site::from_wiki(wiki).unwrap(), &qid);
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn listbuilding(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("listbuilding")
        .expect("No subcommand matches found");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let title = params.get_one::<String>("title").expect("--title missing");
    let mut tool = ListBuilding::new(Site::from_wiki(wiki).unwrap(), title);
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn wikinearby(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("wikinearby")
        .expect("No subcommand matches found");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let title = params.get_one::<String>("title");
    let lat = params.get_one::<f64>("lat");
    let lon = params.get_one::<f64>("lon");
    let offset = params.get_one::<usize>("offset");
    let site = Site::from_wiki(wiki).unwrap();
    let mut tool = match title {
        Some(title) => WikiNearby::new_from_page(site, title),
        None => match (lat, lon) {
            (Some(lat), Some(lon)) => WikiNearby::new_from_coordinates(site, *lat, *lon),
            _ => panic!("Either page title or latitude&longitude are required"),
        },
    };
    if let Some(offset) = offset {
        tool.set_offset(*offset);
    }
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn xtools_pages(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("xtools_pages")
        .expect("No subcommand matches found");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let user = params.get_one::<String>("user").expect("--user missing");
    let namespace_id = params.get_one::<u32>("ns").expect("--ns missing"); // Has default value 0

    let site = Site::from_wiki(wiki).unwrap();
    let mut tool = XtoolsPages::new(site, user).with_namespace_id(*namespace_id);
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn completer(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("completer")
        .expect("No subcommand matches found");

    let from = params.get_one::<String>("from").unwrap();
    let to = params.get_one::<String>("to").unwrap();
    let psid = params.get_one::<String>("psid");
    let template = params.get_one::<String>("template");
    let category = params.get_one::<String>("category");
    let depth = params.get_one::<u32>("depth").unwrap();

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
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn duplicity(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("duplicity")
        .expect("No subcommand matches found");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let mut tool = Duplicity::new(Site::from_wiki(wiki).unwrap());
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn search(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("search")
        .expect("No subcommand matches found");
    let wiki = params.get_one::<String>("wiki").expect("--wiki missing");
    let query = params.get_one::<String>("query").expect("--query missing");
    let mut tool = WikiSearch::new(Site::from_wiki(wiki).unwrap(), query);
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn subset(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("subset")
        .expect("No subcommand matches found");
    let file1 = params.get_one::<String>("file1").expect("--file1 missing");
    let file2 = params.get_one::<String>("file2").expect("--file2 missing");
    let pages1 = PageList::from_file(file1).unwrap();
    let pages2 = PageList::from_file(file2).unwrap();
    let result = pages1.subset(&pages2).await;
    let out = result.as_json().await;
    write_output(&out, params_all);
}

async fn union(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("union")
        .expect("No subcommand matches found");
    let file1 = params.get_one::<String>("file1").expect("--file1 missing");
    let file2 = params.get_one::<String>("file2").expect("--file2 missing");
    let pages1 = PageList::from_file(file1).unwrap();
    let pages2 = PageList::from_file(file2).unwrap();
    let result = pages1.union(&pages2).await;
    let out = result.as_json().await;
    write_output(&out, params_all);
}

async fn pagepile(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("pagepile")
        .expect("No subcommand matches found");
    let id = params.get_one::<u32>("id").expect("--id missing");
    let mut tool = PagePile::new(*id);
    tool.run().await.unwrap();
    let out = tool.as_json().await.unwrap();
    write_output(&out, params_all);
}

async fn petscan(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("petscan")
        .expect("No subcommand matches found");
    let id = params.get_one::<u32>("id").expect("--id missing");
    let override_params = params
        .get_many::<String>("params")
        .unwrap_or_default()
        .collect::<Vec<_>>();
    let mut tool = PetScan::new(*id);
    for p in override_params {
        let mut parts = p.splitn(2, "=");
        let key = parts.next().expect("Override parameter key expected");
        let value = parts.next().expect("Override parameter value expected");
        if key == "format" {
            eprintln!("Ignoring format override");
            continue;
        }
        tool.parameters_mut().retain(|(k, _)| k != key); // Remove old value, if any
        tool.parameters_mut()
            .push((key.to_string(), value.to_string())); // Add new value
    }
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn missing_topics(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("missing_topics")
        .expect("No subcommand matches found");

    let wiki = params.get_one::<String>("wiki").unwrap();
    let article = params.get_one::<String>("article");
    let category = params.get_one::<String>("category");
    let depth = *params.get_one::<u32>("depth").unwrap();
    let no_template_links = params
        .get_one::<bool>("no_template_links")
        .copied()
        .unwrap_or_default();

    let mut tool = MissingTopics::new(Site::from_wiki(wiki).expect("No such wiki {wiki}"))
        .no_template_links(no_template_links);
    if let Some(article) = article {
        tool = tool.with_article(article);
    }
    if let Some(category) = category {
        tool = tool.with_category(category, depth);
    }
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

async fn grep(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("grep")
        .expect("No subcommand matches found");

    let wiki = params.get_one::<String>("wiki").expect("--wiki required");
    let pattern = params
        .get_one::<String>("pattern")
        .expect("--pattern required");
    let namespace_id = params.get_one::<usize>("ns").unwrap();

    let mut tool = Grep::new(Site::from_wiki(wiki).expect("No such wiki {wiki}"), pattern)
        .with_namespace(*namespace_id);
    tool.run().await.unwrap();
    let out = tool.as_json().await;
    write_output(&out, params_all);
}

fn get_arg_matches() -> ArgMatches {
    Command::new("Tools Interface")
        .author("Magnus Manske <magnusmanske@googlemail.com>")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Runs queries against various Wikimedia tools")
        .arg(
            Arg::new("format")
                .default_value("json")
                .long("format")
                .help("Output format (optional)"),
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
                        .required(false),
                )
                .arg(
                    Arg::new("lon")
                        .long("lon")
                        .help("Longitude (requires --lat)")
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
            Command::new("subset")
                .about("Generates the subset of two JSON output files. Merges metadata for duplicate pages")
                .arg(Arg::new("file1").required(true).index(1))
                .arg(Arg::new("file2").required(true).index(2)),
            Command::new("union")
                .about("Generates the union of two JSON output files. Merges metadata for duplicate pages")
                .arg(Arg::new("file1").required(true).index(1))
                .arg(Arg::new("file2").required(true).index(2)),
        ])
        .get_matches()
}

#[tokio::main]
async fn main() {
    let m = get_arg_matches();
    match m.subcommand_name() {
        Some("alistbuildingtool") => alistbuildingtool(&m).await,
        Some("completer") => completer(&m).await,
        Some("duplicity") => duplicity(&m).await,
        Some("grep") => grep(&m).await,
        Some("listbuilding") => listbuilding(&m).await,
        Some("missing_topics") => missing_topics(&m).await,
        Some("pagepile") => pagepile(&m).await,
        Some("petscan") => petscan(&m).await,
        Some("search") => search(&m).await,
        Some("subset") => subset(&m).await,
        Some("union") => union(&m).await,
        Some("wikinearby") => wikinearby(&m).await,
        Some("xtools_pages") => xtools_pages(&m).await,
        Some(other) => eprintln!("Unknown subcommand given: {other}"),
        None => eprintln!("No subcommand given"),
    }
}
