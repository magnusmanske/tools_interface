# Tools Interface

This rust crate implements structs to easily interface with several Wikipedia/Wikidata/Wikimedia tools and APIs.

It is available as a Rust [crate](https://crates.io/crates/tools_interface).

## Supported tools

- [A List Building Tool](https://a-list-bulding-tool.toolforge.org/)
- [Completer](https://completer.toolforge.org/)
- [Duplicity](https://wikidata-todo.toolforge.org/duplicity/)
- [Grep](https://grep.toolforge.org/index.php)
- [List Building](https://list-building.toolforge.org)
- [Missing Topics](https://missingtopics.toolforge.org/)
- [PagePile](https://pagepile.toolforge.org/) (read only)
- [Pageviews API](https://wikitech.wikimedia.org/wiki/Analytics/AQS/Pageviews)
- [Persondata Template](https://persondata.toolforge.org/vorlagen/)
- [PetScan](https://petscan.wmflabs.org/)
- [Quarry](https://quarry.wmcloud.org/) (retrieve existing results only)
- [QuickStatements](https://quickstatements.toolforge.org/) (create and start batches)
- [SparqlRC](https://wikidata-todo.toolforge.org/sparql_rc.php)
- [WikiNearby](https://wikinearby.toolforge.org/)
- [XTools pages](https://xtools.wmcloud.org/pages)

If you would like to see other tools supported, add a request to the [Issue tracker](https://github.com/magnusmanske/tools_interface/issues).

## Composing results

Every tool result becomes a *page list*: pages on one wiki, each carrying the metadata of
the tools that contributed it. Lists can be combined and moved between wikis:

- *union*, *intersection*, *not-in* and *sym-diff* of two or more lists
- *cast* a list to another wiki via Wikidata sitelinks, and report which pages have no
  counterpart there
- *search* on any WMF wiki
- write a list back out as a new PagePile, or as a QuickStatements batch

Metadata is namespaced by the tool that produced it, so combining lists never loses it.

## List file format

Lists are JSONL: one JSON object per line, so they can be piped between processes.
The first line is a header with the list's site and provenance:

```json
{"ti":1,"site":{"wiki":"enwiki","language":"en","project":"wikipedia"},"sources":[{"tool":"petscan"}]}
```

Every following line is one page:

```json
{"wiki":"enwiki","namespace_id":0,"title":"Earth","prefixed_title":"Earth","meta":{"petscan":{"counter":3}}}
```

Each page repeats its own `wiki`, so a list still works after a `jq` filter has dropped
the header line; only the `sources` provenance is lost. A page's identity is its
`namespace_id` plus `title` — `prefixed_title` is a convenience.

The pre-0.2.0 format (one JSON document with a `pages` array) is still read, and can
still be written with `--format json`.

## Binary

There is a `ti` binary, working as a command-line interface to the tools_interface library.
It allows you to run queries against various Wikimedia tools from shell.

### Installation
To just use the binary, follow these steps:
```
# Install rust, unless it is already installed
# See https://rust-lang.org/tools/install/
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install `ti`
cargo install tools_interface
```

### Usage
Use `ti help` to get the list of subommands,
and `ti help <subcommand>` to get help on a specific subcommand.

Output is JSONL by default (see *List file format* above), so results can be piped into
another `ti` command, into `jq`, or into `wc -l`.
`--format titles` writes just the prefixed titles, one per line, and `--format json`
writes the pre-0.2.0 layout.

Example: Run a PetScan query with a known PSID, and override two parameters:

```shell
ti petscan --id 28348714 --params "foo=bar" "baz=123"
```

Example: Run Missing Topics on German Wikipedia for the article "Biologie", without template links:

```shell
ti missing_topics --wiki dewiki --article Biologie --no_template_links
```

### Combining lists

`-` stands for stdin, so lists can be built up in a pipeline:

```shell
# Pages a user created that are not in a PagePile
ti xtools_pages --wiki enwiki --user Magnus_Manske > mine.jsonl
ti pagepile --id 51805 > pile.jsonl
ti not-in mine.jsonl pile.jsonl

# Lists on different wikis are cast to the wiki of the first one automatically,
# so the above works even though that PagePile is on dewiki.

# German articles that have no English counterpart, as a new PagePile
ti petscan --id 28348714 | ti cast --wiki enwiki --missing - | ti pagepile_create -

# Add P31:Q5 to the items behind a list of articles
ti petscan --id 28348714 | ti cast --wiki wikidatawiki - \
  | ti quickstatements --user YOU --token YOUR_TOKEN --command '{item}\tP31\tQ5'
```

Set operations preserve the order of the first list, so a ranking produced by a tool
(by page views, by link count, ...) survives being filtered against another list.

### Filtering with `jq`

```shell
# Just the page titles
ti SOME_COMMAND --format titles

# Using PetScan to get a category tree of all churches in Germany, and their Wikidata items:
ti petscan --id 39413398 | jq -r 'select(.title) | "\(.prefixed_title)\t\(.meta.petscan.metadata.wikidata)"'

# Filter, and keep the result readable by ti
ti SOME_COMMAND | jq -c 'select(.title and .meta.pageviews.views > 1000)' | ti cat
```

Guard `jq` filters with `.title`, which only pages have, so they skip the header line.
`ti cat` reads a list and writes it out again: it converts between `--format` values,
validates input, and puts a fresh header on a list that `jq` has stripped.
