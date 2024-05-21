//! # Persondata Vorlagen
//! Queries the [Persondata Vorlagen tool](https://persondata.toolforge.org/vorlagen) for information about template usage on Germam Wikipedia.
//! Build a `PersondataTemplatesQuery` and call `get_blocking()` to get the results.
//! Results are returned as a `Vec<PersondataTemplatesResult>`.
//!
//! Example:
//! ```rust
//! let results: Vec<PersondataTemplatesResult> = PersondataTemplatesQuery::with_template("Roscher")
//!     .parameter_name("4")
//!     .get().await.unwrap();
//! ```

use crate::ToolsError;
use std::{collections::HashMap, fmt};

#[derive(Debug, Default, PartialEq)]
pub enum PersondataTemplatesOccOp {
    #[default]
    Equal,
    Larger,
    Smaller,
}

impl fmt::Display for PersondataTemplatesOccOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersondataTemplatesOccOp::Equal => write!(f, "eq"),
            PersondataTemplatesOccOp::Larger => write!(f, "gt"),
            PersondataTemplatesOccOp::Smaller => write!(f, "lt"),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum PersondataTemplatesParamValueOp {
    #[default]
    Equal,
    Contains,
    Like,
    NotLike,
    Regexp,
    NotRegexp,
}

impl fmt::Display for PersondataTemplatesParamValueOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersondataTemplatesParamValueOp::Equal => write!(f, "eq"),
            PersondataTemplatesParamValueOp::Contains => write!(f, "hs"),
            PersondataTemplatesParamValueOp::Like => write!(f, "lk"),
            PersondataTemplatesParamValueOp::NotLike => write!(f, "nl"),
            PersondataTemplatesParamValueOp::Regexp => write!(f, "rx"),
            PersondataTemplatesParamValueOp::NotRegexp => write!(f, "nr"),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum PersondataTemplatesParamNameOp {
    #[default]
    Equal,
    Unequal,
    Missing,
    Like,
    NotLike,
    Regexp,
    NotRegexp,
}

impl fmt::Display for PersondataTemplatesParamNameOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersondataTemplatesParamNameOp::Equal => write!(f, "eq"),
            PersondataTemplatesParamNameOp::Unequal => write!(f, "ne"),
            PersondataTemplatesParamNameOp::Missing => write!(f, "miss"),
            PersondataTemplatesParamNameOp::Like => write!(f, "lk"),
            PersondataTemplatesParamNameOp::NotLike => write!(f, "nl"),
            PersondataTemplatesParamNameOp::Regexp => write!(f, "rx"),
            PersondataTemplatesParamNameOp::NotRegexp => write!(f, "nr"),
        }
    }
}

#[derive(Debug, Default)]
pub struct PersondataTemplatesResult {
    article: String,
    usage_number: u32,
    params: HashMap<u32, String>,
}

impl PersondataTemplatesResult {
    fn from_record(header: &csv::StringRecord, record: &csv::StringRecord) -> Self {
        let mut result = Self::default();
        for i in 0..header.len() {
            match record.get(i) {
                Some(value) => {
                    match header.get(i) {
                        Some("Artikel") => result.article = value.to_string(),
                        Some("Einbindung") => result.usage_number = value.parse().unwrap_or(0),
                        Some(other) => {
                            if let Ok(key) = other.parse::<u32>() {
                                result.params.insert(key, value.to_string());
                            } else {
                                //     println!("Unknown header: {other}:{value}");
                            }
                        }
                        _ => {}
                    }
                }
                None => (),
            }
        }
        result
    }

    pub fn article(&self) -> &str {
        &self.article
    }

    /// "Einbindung"
    pub fn usage_number(&self) -> u32 {
        self.usage_number
    }

    pub fn params(&self) -> &HashMap<u32, String> {
        &self.params
    }
}

#[derive(Debug, Default)]
pub struct PersondataTemplatesQuery {
    with_wl: bool,                                   // Mit Weiterleitungen
    tmpl: String,                                    // Name der Vorlage
    occ: Option<u32>,                                // Einschränkung auf die wievielte Einbindung
    occ_op: PersondataTemplatesOccOp,                // Vergleichs-Operator
    in_t: bool,         // Nur Vorlage die direkt in einer Tabelle enthalten sind
    in_v: bool,         // Nur Vorlage die direkt in einer anderen Vorlage enthalten sind
    in_r: bool,         // Nur Vorlage die direkt in einer Referenz enthalten sind
    in_l: bool,         // Nur Vorlage die direkt in einem Wikilink (Datei:) enthalten sind
    in_a: bool,         // Nur Vorlage die direkt in einem Artikel enthalten sind
    param_name: String, // Name des Vorlagen-Parameters, mehrere Parameter können durch Pipe-Zeichen getrennt werden (nur bei Vergleich auf 'Gleich', 'Ungleich', 'Like' und 'NOT Like')
    param_name_op: PersondataTemplatesParamNameOp, // Vergleichs-Operator
    param_value: String, // Name des Vorlagen-Parameters, mehrere Parameter können durch Pipe-Zeichen getrennt werden (nur bei Vergleich auf 'Gleich', 'Ungleich', 'Like' und 'NOT Like')
    param_value_op: PersondataTemplatesParamValueOp, // Vergleichs-Operator
    in_c: bool,          // Text innerhalb HTML-Kommentaren des Parameterinhalts durchsuchen
    case: bool,          // Groß-/Kleinschreibung im Parameterinhalt beachten
}

impl PersondataTemplatesQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_template<S: Into<String>>(tmpl: S) -> Self {
        Self {
            tmpl: tmpl.into(),
            with_wl: true,
            ..Default::default()
        }
    }

    pub fn with_occurrence(self, occ: u32) -> Self {
        self.with_occurrence_op(occ, PersondataTemplatesOccOp::default())
    }

    pub fn with_occurrence_op(self, occ: u32, occ_op: PersondataTemplatesOccOp) -> Self {
        Self {
            occ: Some(occ),
            occ_op,
            ..self
        }
    }

    pub fn in_table(self) -> Self {
        Self { in_t: true, ..self }
    }

    pub fn in_template(self) -> Self {
        Self { in_v: true, ..self }
    }

    pub fn in_reference(self) -> Self {
        Self { in_r: true, ..self }
    }

    pub fn in_wikilink(self) -> Self {
        Self { in_l: true, ..self }
    }

    pub fn in_article(self) -> Self {
        Self { in_a: true, ..self }
    }

    pub fn in_comments(self) -> Self {
        Self { in_c: true, ..self }
    }

    pub fn case_sensitive(self) -> Self {
        Self { case: true, ..self }
    }

    pub fn parameter_name<S: Into<String>>(self, param_name: S) -> Self {
        self.parameter_name_op(param_name, PersondataTemplatesParamNameOp::default())
    }

    pub fn parameter_name_op<S: Into<String>>(
        self,
        param_name: S,
        param_name_op: PersondataTemplatesParamNameOp,
    ) -> Self {
        Self {
            param_name: param_name.into(),
            param_name_op,
            ..self
        }
    }

    pub fn parameter_value<S: Into<String>>(self, param_value: S) -> Self {
        self.parameter_value_op(param_value, PersondataTemplatesParamValueOp::default())
    }

    pub fn parameter_value_op<S: Into<String>>(
        self,
        param_value: S,
        param_value_op: PersondataTemplatesParamValueOp,
    ) -> Self {
        Self {
            param_value: param_value.into(),
            param_value_op,
            ..self
        }
    }

    fn generate_csv_url(&self) -> String {
        let mut url = "https://persondata.toolforge.org/vorlagen/index.php?export=1&tzoffset=0&show_occ&show_param&show_value".to_string();

        if !self.tmpl.is_empty() {
            url += &format!("&tmpl={}", self.tmpl);
            if self.with_wl {
                url += "&with_wl";
            }
        }

        if let Some(occ) = self.occ {
            url += &format!("&occ={occ}");
            if self.occ_op != PersondataTemplatesOccOp::default() {
                url += &format!("&occ_op={}", self.occ_op);
            }
        }

        if !self.param_name.is_empty() {
            url += &format!("&param={}", self.param_name);
            if self.param_name_op != PersondataTemplatesParamNameOp::default() {
                url += &format!("&param_name_op={}", self.param_name_op);
            }
        }

        if !self.param_value.is_empty() {
            url += &format!("&value={}", self.param_value);
            if self.param_value_op != PersondataTemplatesParamValueOp::default() {
                url += &format!("&param_value_op={}", self.param_value_op);
            }
        }

        if self.in_t {
            url += "&in_t";
        }
        if self.in_v {
            url += "&in_v";
        }
        if self.in_r {
            url += "&in_r";
        }
        if self.in_l {
            url += "&in_l";
        }
        if self.in_a {
            url += "&in_a";
        }
        if self.in_c {
            url += "&in_c";
        }
        if self.case {
            url += "&case";
        }

        url
    }

    #[cfg(feature = "blocking")]
    pub fn get_blocking(&self) -> Result<Vec<PersondataTemplatesResult>, ToolsError> {
        let url = self.generate_csv_url();
        let client = crate::ToolsInterface::blocking_client()?;
        let response = client.get(&url).send()?;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .flexible(true)
            .from_reader(response);
        let headers = reader.headers()?.to_owned();

        Ok(reader
            .records()
            .filter_map(|result| result.ok())
            .map(|record| PersondataTemplatesResult::from_record(&headers, &record))
            .collect())
    }

    #[cfg(feature = "tokio")]
    pub async fn get(&self) -> Result<Vec<PersondataTemplatesResult>, ToolsError> {
        let url = self.generate_csv_url();
        let client = crate::ToolsInterface::tokio_client()?;
        let response = client.get(&url).send().await?;
        let body = response.text().await?;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .flexible(true)
            .from_reader(body.as_bytes());
        let headers = reader.headers()?.to_owned();

        Ok(reader
            .records()
            .filter_map(|result| result.ok())
            .map(|record| PersondataTemplatesResult::from_record(&headers, &record))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persondata_templates_query() {
        let query = PersondataTemplatesQuery::with_template("Roscher")
            .with_occurrence_op(4, PersondataTemplatesOccOp::Equal)
            .in_table()
            .in_template()
            .in_reference()
            .in_wikilink()
            .in_article()
            .in_comments()
            .case_sensitive()
            .parameter_name_op("name", PersondataTemplatesParamNameOp::Equal)
            .parameter_value_op("value", PersondataTemplatesParamValueOp::Equal);

        assert_eq!(query.tmpl, "Roscher");
        assert_eq!(query.with_wl, true);
        assert_eq!(query.occ, Some(4));
        assert_eq!(query.occ_op, PersondataTemplatesOccOp::Equal);
        assert_eq!(query.in_t, true);
        assert_eq!(query.in_v, true);
        assert_eq!(query.in_r, true);
        assert_eq!(query.in_l, true);
        assert_eq!(query.in_a, true);
        assert_eq!(query.in_c, true);
        assert_eq!(query.case, true);
        assert_eq!(query.param_name, "name");
        assert_eq!(query.param_name_op, PersondataTemplatesParamNameOp::Equal);
        assert_eq!(query.param_value, "value");
        assert_eq!(query.param_value_op, PersondataTemplatesParamValueOp::Equal);
    }

    #[cfg(feature = "blocking")]
    #[test]
    fn get_persondata_template_blocking() {
        let query = PersondataTemplatesQuery::with_template("Roscher")
            .parameter_name_op("4", PersondataTemplatesParamNameOp::default());
        let x = query.get_blocking().unwrap();
        assert!(x.len() > 2000);
    }

    #[cfg(feature = "tokio")]
    #[tokio::test]
    async fn get_persondata_template_async() {
        let query = PersondataTemplatesQuery::with_template("Roscher")
            .parameter_name_op("4", PersondataTemplatesParamNameOp::default());
        let x = query.get().await.unwrap();
        assert!(x.len() > 2000);
    }
}
