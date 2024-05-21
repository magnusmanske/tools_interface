//! # Site
//! `Site` is a struct that represents a MediaWiki site.
//! It can be created from a wiki name, or a language and project.
//! It provides methods to get the language, project, and webserver of the site,
//! as well as a MediaWiki `Api` object.

use lazy_static::lazy_static;
use mediawiki::api::Api;
use regex::Regex;

use crate::ToolsError;

lazy_static! {
    static ref RE_WIKI: Regex = Regex::new(r"^(.+?)(wik.+)$").expect("Regex error");
    static ref RE_WEBSERVER_WIKIPEDIA: Regex = Regex::new(r"^(.+)wiki$").expect("Regex error");
    static ref RE_WEBSERVER_WIKI: Regex = Regex::new(r"^(.+)(wik.+)$").expect("Regex error");
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Site {
    wiki: String,
    language: String,
    project: String,
}

impl Site {
    /// Creates a new `Site` object from a wiki name.
    /// Returns `None` if the wiki name is not recognized.
    pub fn from_wiki(wiki: &str) -> Option<Self> {
        let wiki = Self::normalize_wiki(wiki);
        let language;
        let mut project;
        match wiki.as_str() {
            "commonswiki" => {
                language = "commons".to_string();
                project = "wikimedia".to_string();
            }
            "wikidatawiki" => {
                language = "www".to_string();
                project = "wikidata".to_string();
            }
            "specieswiki" => {
                language = "species".to_string();
                project = "wikimedia".to_string();
            }
            "metawiki" => {
                language = "meta".to_string();
                project = "wikimedia".to_string();
            }
            _ => {
                match RE_WIKI.captures(&wiki) {
                    Some(cap) => {
                        language = cap.get(1)?.as_str().to_string();
                        project = cap.get(2)?.as_str().to_string();
                    }
                    None => return None,
                }
                if project == "wiki" {
                    project = "wikipedia".to_string();
                } else if project.ends_with("wiki") {
                    project = project[..project.len() - 4].to_string();
                }
            }
        }

        Some(Self {
            wiki,
            language,
            project,
        })
    }

    /// Creates a new `Site` object from a language and project.
    pub fn from_language_project(language: &str, project: &str) -> Self {
        let wiki = match project {
            "wikimedia" => format!("{}wiki", language),
            "wikidata" => "wikidatawiki".to_string(),
            "wikipedia" => format!("{}wiki", language),
            _ => format!("{}{}wiki", language, project),
        };
        Self {
            wiki,
            language: language.to_string(),
            project: project.to_string(),
        }
    }

    fn normalize_wiki(wiki: &str) -> String {
        wiki.replace("-", "_").trim().to_ascii_lowercase()
    }

    /// Returns "language.project", with language having "-" instead of "_".
    /// Useful for `Pageviews``.
    pub fn language_project(&self) -> String {
        format!(
            "{language}.{project}",
            language = self.language.replace('_', "-"),
            project = self.project
        )
    }

    /// Returns the webserver for the site, e.g. "en.wikipedia.org".
    pub fn webserver(&self) -> String {
        format!(
            "{language}.{project}.org",
            language = self.language.replace('_', "-"),
            project = self.project
        )
    }

    /// Returns the wiki name, e.g. "enwiki".
    pub fn wiki(&self) -> &str {
        &self.wiki
    }

    /// Returns the language code, e.g. "en".
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Returns the project name, e.g. "wikipedia".
    pub fn project(&self) -> &str {
        &self.project
    }

    /// Returns a MediaWiki `Api` object for the site.
    pub async fn api(&self) -> Result<Api, ToolsError> {
        let api_url = format!(
            "https://{webserver}/w/api.php",
            webserver = self.webserver()
        );
        let api = Api::new(&api_url).await?;
        Ok(api)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_from_wiki() {
        let site = Site::from_wiki("enwiki").unwrap();
        assert_eq!(site.wiki, "enwiki");
        assert_eq!(site.language, "en");
        assert_eq!(site.project, "wikipedia");

        let site = Site::from_wiki("enwiktionarywiki").unwrap();
        assert_eq!(site.wiki, "enwiktionarywiki");
        assert_eq!(site.language, "en");
        assert_eq!(site.project, "wiktionary");

        let site = Site::from_wiki("commonswiki").unwrap();
        assert_eq!(site.wiki, "commonswiki");
        assert_eq!(site.language, "commons");
        assert_eq!(site.project, "wikimedia");

        let site = Site::from_wiki("wikidatawiki").unwrap();
        assert_eq!(site.wiki, "wikidatawiki");
        assert_eq!(site.language, "www");
        assert_eq!(site.project, "wikidata");

        let site = Site::from_wiki("specieswiki").unwrap();
        assert_eq!(site.wiki, "specieswiki");
        assert_eq!(site.language, "species");
        assert_eq!(site.project, "wikimedia");

        let site = Site::from_wiki("metawiki").unwrap();
        assert_eq!(site.wiki, "metawiki");
        assert_eq!(site.language, "meta");
        assert_eq!(site.project, "wikimedia");
    }

    #[test]
    fn test_site_from_language_project() {
        let site = Site::from_language_project("en", "wikipedia");
        assert_eq!(site.wiki, "enwiki");
        assert_eq!(site.language, "en");
        assert_eq!(site.project, "wikipedia");

        let site = Site::from_language_project("en", "wiktionary");
        assert_eq!(site.wiki, "enwiktionarywiki");
        assert_eq!(site.language, "en");
        assert_eq!(site.project, "wiktionary");

        let site = Site::from_language_project("commons", "wikimedia");
        assert_eq!(site.wiki, "commonswiki");
        assert_eq!(site.language, "commons");
        assert_eq!(site.project, "wikimedia");

        let site = Site::from_language_project("www", "wikidata");
        assert_eq!(site.wiki, "wikidatawiki");
        assert_eq!(site.language, "www");
        assert_eq!(site.project, "wikidata");

        let site = Site::from_language_project("species", "wikimedia");
        assert_eq!(site.wiki, "specieswiki");
        assert_eq!(site.language, "species");
        assert_eq!(site.project, "wikimedia");

        let site = Site::from_language_project("meta", "wikimedia");
        assert_eq!(site.wiki, "metawiki");
        assert_eq!(site.language, "meta");
        assert_eq!(site.project, "wikimedia");
    }
}
