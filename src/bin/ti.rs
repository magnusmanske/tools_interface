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

use clap::{value_parser, Arg, ArgAction, ArgMatches, Command};
use mediawiki::{api::Api, title::Title};
use serde_json::{json, Value};
use tools_interface::{Completer, CompleterFilter, MissingTopics, PagePile, PetScan, Site};

#[derive(Debug, PartialEq)]
struct FancyTitle {
    title: Title,
    prefixed_title: String,
}

impl FancyTitle {
    fn new(s: &str, ns: i64, api: &Api) -> Self {
        let title = Title::new(s, ns);
        Self {
            prefixed_title: title.full_pretty(&api).unwrap_or_default(),
            title,
        }
    }

    fn from_prefixed(s: &str, api: &Api) -> Self {
        let title = Title::new_from_full(s, api);
        Self {
            prefixed_title: title.full_pretty(api).unwrap_or_default(),
            title,
        }
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "title": self.title.pretty(),
            "prefixed_title": self.prefixed_title,
            "namespace_id": self.title.namespace_id(),
        })
    }
}

fn write_json(j: &Value) {
    println!("{}", serde_json::to_string_pretty(&j).unwrap());
}

fn write_output(out: &Value, params_all: &ArgMatches) {
    let format = params_all
        .get_one::<String>("format")
        .expect("--format missing");
    match format.as_str() {
        "json" => write_json(&out),
        _ => eprintln!("Unknown format: {format}"),
    }
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

    let mut c = Completer::new(&from, &to);
    if let Some(psid) = psid {
        c = c.filter(CompleterFilter::PetScan {
            psid: psid.to_string(),
        });
    }
    if let Some(template) = template {
        c = c.filter(CompleterFilter::Template {
            template: template.to_string(),
        });
    }
    if let Some(category) = category {
        c = c.filter(CompleterFilter::Category {
            category: category.to_string(),
            depth: *depth,
        });
    }
    c.run().await.unwrap();

    let site = Site::from_language_project(&to, "wikipedia");
    let api = site.api().await.unwrap();
    let out = json!({
        "pages": c.results()
            .iter()
            .map(|(prefixed_title,counter)| (FancyTitle::from_prefixed(&prefixed_title, &api).to_json(),counter))
            .map(|(mut v,counter)| {v["counter"] = json!(*counter); v})
            .collect::<Vec<Value>>(),
        "site": site,
    });
    write_output(&out, params_all);
}

async fn pagepile(params_all: &ArgMatches) {
    let params = params_all
        .subcommand_matches("pagepile")
        .expect("No subcommand matches found");
    let id = params.get_one::<u32>("id").expect("--id missing");
    let mut pp = PagePile::new(*id);
    pp.get().await.unwrap();
    let site = pp.site().expect("Unknown site for PagePile");
    let api = site.api().await.unwrap();
    let out = json!({
        "pages": pp.prefixed_titles()
            .iter()
            .map(|prefixed_title| FancyTitle::from_prefixed(&prefixed_title, &api).to_json())
            .collect::<Vec<Value>>(),
        "site": site,
    });
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
    let mut ps = PetScan::new(*id);
    for p in override_params {
        let mut parts = p.splitn(2, "=");
        let key = parts.next().expect("Override parameter key expected");
        let value = parts.next().expect("Override parameter value expected");
        if key == "format" {
            eprintln!("Ignoring format override");
            continue;
        }
        ps.parameters_mut().retain(|(k, _)| k != key); // Remove old value, if any
        ps.parameters_mut()
            .push((key.to_string(), value.to_string())); // Add new value
    }
    ps.get().await.unwrap();
    let site = Site::from_wiki(ps.wiki().expect("No wiki in PetScan result")).unwrap();
    let api = site.api().await.unwrap();
    let out = json!({
        "pages": ps.pages()
            .iter()
            .map(|page| (FancyTitle::new(&page.page_title, page.page_namespace, &api).to_json(),page))
            .map(|(mut j,page)| {
                j["id"] = page.page_id.into();
                j["len"] = page.page_len.into();
                j["timestamp"] = json!(page.page_latest);
                j["metadata"] = json!(page.metadata);
                j["giu"] = json!(page.giu); // Global image usage
                j
            })
            .collect::<Vec<Value>>(),
        "site": site,
    });
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
        .map(|b| *b)
        .unwrap_or_default();

    let mut mt = MissingTopics::new(Site::from_wiki(&wiki).expect("No such wiki {wiki}"))
        .no_template_links(no_template_links);
    if let Some(article) = article {
        mt = mt.with_article(&article);
    }
    if let Some(category) = category {
        mt = mt.with_category(category, depth);
    }

    mt.run().await.unwrap();

    let site = mt.site();
    let api = site.api().await.unwrap();
    let out = json!({
        "pages": mt.results()
            .iter()
            .map(|(prefixed_title,counter)| (FancyTitle::from_prefixed(&prefixed_title, &api).to_json(),counter))
            .map(|(mut v,counter)| {v["counter"] = json!(*counter); v})
            .collect::<Vec<Value>>(),
        "site": site,
    });
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
        ])
        .get_matches()
}

#[tokio::main]
async fn main() {
    let m = get_arg_matches();
    match m.subcommand_name() {
        Some("completer") => completer(&m).await,
        Some("pagepile") => pagepile(&m).await,
        Some("petscan") => petscan(&m).await,
        Some("missing_topics") => missing_topics(&m).await,
        _ => eprintln!("No subcommand given"),
    }
}

/*
TODO:

[Pageviews API](https://wikitech.wikimedia.org/wiki/Analytics/AQS/Pageviews)
[Persondata Template](https://persondata.toolforge.org/vorlagen/)
[Quarry](https://quarry.wmcloud.org/) (existing results only)
[QuickStatements](https://quickstatements.toolforge.org/) (start batches)
[SparqlRC](https://wikidata-todo.toolforge.org/sparql_rc.php)

*/
