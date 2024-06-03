# Tools Interface

This rust crate implements structs to easily interface with several Wikipedia/Wikidata/Wikimedia tools and APIs.

## Supported tools

- [A List Building Tool](https://a-list-bulding-tool.toolforge.org/)
- [Completer](https://completer.toolforge.org/)
- [Duplicity](https://wikidata-todo.toolforge.org/duplicity/)
- [PagePile](https://pagepile.toolforge.org/) (read only)
- [Pageviews API](https://wikitech.wikimedia.org/wiki/Analytics/AQS/Pageviews)
- [Persondata Template](https://persondata.toolforge.org/vorlagen/)
- [PetScan](https://petscan.wmflabs.org/)
- [Missing Topics](https://missingtopics.toolforge.org/)
- [Quarry](https://quarry.wmcloud.org/) (existing results only)
- [QuickStatements](https://quickstatements.toolforge.org/) (start batches)
- [SparqlRC](https://wikidata-todo.toolforge.org/sparql_rc.php)

If you would like to see other tools supported, add a request to the [Issue tracker](https://github.com/magnusmanske/tools_interface/issues).

## Binary

The is a `ti` binary, working as a command-line interface to the tools_interface library.
It allows you to run queries against various Wikimedia tools from shell.

Use `ti help` to get the list of subommands,
and `ti help <subcommand>` to get help on a specific subcommand.

Default output format is JSON, so you can pipe the output to `jq` for downstream processing.
Pages are listed in the `.pages` array, with each page having a `title`, a `prefixed_title`, and a `namespace_id`.
Each page can have additional fields, depending on the tool used.
The `.site` object contains the result site's wiki, language and project.

Example: Run a PetScan query with a known PSID, and override two parameters:

```shell
ti petscan --id 28348714 --params "foo=bar" "baz=123"
```

Example: Run Missing Topics on German Wikipedia for the article "Biologie", without template links:

```shell
ti missing_topics --wiki dewiki --article Biologie --no_template_links
```

To convert the output to a more human-readable format, you can use `jq`:

```shell
# First, pipe your output to a file:
ti SOME_COMMAND > test.json
# Assuming you just want the page titles:
jq -r '.pages[].prefixed_title' < test.json
# Assuming the output has additional `counter` fields:
jq -r '.pages[] | "\(.prefixed_title)\t\(.counter)"' < test.json
```
