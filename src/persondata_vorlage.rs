use std::{collections::HashMap, fmt};
use crate::ToolsError;

static TOOLS_INTERFACE_USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
);


#[derive(Debug, Default, PartialEq)]
pub enum PersondataVorlagenOccOp {
    #[default]
    Equal,
    Larger,
    Smaller,
}

impl fmt::Display for PersondataVorlagenOccOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersondataVorlagenOccOp::Equal => write!(f, "eq"),
            PersondataVorlagenOccOp::Larger => write!(f, "gt"),
            PersondataVorlagenOccOp::Smaller => write!(f, "lt"),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum PersondataVorlagenParamValueOp {
    #[default]
    Equal,
    Contains,
    Like,
    NotLike,
    Regexp,
    NotRegexp,
}

impl fmt::Display for PersondataVorlagenParamValueOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersondataVorlagenParamValueOp::Equal => write!(f, "eq"),
            PersondataVorlagenParamValueOp::Contains => write!(f, "hs"),
            PersondataVorlagenParamValueOp::Like => write!(f, "lk"),
            PersondataVorlagenParamValueOp::NotLike => write!(f, "nl"),
            PersondataVorlagenParamValueOp::Regexp => write!(f, "rx"),
            PersondataVorlagenParamValueOp::NotRegexp => write!(f, "nr"),
        }
    }
}

#[derive(Debug, Default, PartialEq)]
pub enum PersondataVorlagenParamNameOp {
    #[default]
    Equal,
    Unequal,
    Missing,
    Like,
    NotLike,
    Regexp,
    NotRegexp,
}

impl fmt::Display for PersondataVorlagenParamNameOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersondataVorlagenParamNameOp::Equal => write!(f, "eq"),
            PersondataVorlagenParamNameOp::Unequal => write!(f, "ne"),
            PersondataVorlagenParamNameOp::Missing => write!(f, "miss"),
            PersondataVorlagenParamNameOp::Like => write!(f, "lk"),
            PersondataVorlagenParamNameOp::NotLike => write!(f, "nl"),
            PersondataVorlagenParamNameOp::Regexp => write!(f, "rx"),
            PersondataVorlagenParamNameOp::NotRegexp => write!(f, "nr"),
        }
    }
}

#[derive(Debug, Default)]
pub struct PersondataVorlagenResult {
    article: String,
    usage_number: u32,
    params: HashMap<u32,String>,
}

impl PersondataVorlagenResult {
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
                        },
                        _ => {},
                    }
                },
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
    
    pub fn params(&self) -> &HashMap<u32,String> {
        &self.params
    }
}

#[derive(Debug, Default)]
pub struct PersondataVorlagenQuery {
    with_wl: bool, // Mit Weiterleitungen
    tmpl: String, // Name der Vorlage
    occ: Option<u32>, // Einschränkung auf die wievielte Einbindung
    occ_op: PersondataVorlagenOccOp, // Vergleichs-Operator
    in_t: bool, // Nur Vorlage die direkt in einer Tabelle enthalten sind
    in_v: bool, // Nur Vorlage die direkt in einer anderen Vorlage enthalten sind
    in_r: bool, // Nur Vorlage die direkt in einer Referenz enthalten sind
    in_l: bool, // Nur Vorlage die direkt in einem Wikilink (Datei:) enthalten sind
    in_a: bool, // Nur Vorlage die direkt in einem Artikel enthalten sind
    param_name: String, // Name des Vorlagen-Parameters, mehrere Parameter können durch Pipe-Zeichen getrennt werden (nur bei Vergleich auf 'Gleich', 'Ungleich', 'Like' und 'NOT Like')
    param_name_op: PersondataVorlagenParamNameOp, // Vergleichs-Operator
    param_value: String, // Name des Vorlagen-Parameters, mehrere Parameter können durch Pipe-Zeichen getrennt werden (nur bei Vergleich auf 'Gleich', 'Ungleich', 'Like' und 'NOT Like')
    param_value_op: PersondataVorlagenParamValueOp, // Vergleichs-Operator
    in_c: bool, // Text innerhalb HTML-Kommentaren des Parameterinhalts durchsuchen
    case: bool, // Groß-/Kleinschreibung im Parameterinhalt beachten
}

impl PersondataVorlagenQuery {
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

    pub fn with_occurrence(self, occ: u32, occ_op: PersondataVorlagenOccOp) -> Self {
        Self { occ: Some(occ), occ_op, ..self }
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

    pub fn parameter_name<S: Into<String>>(self, param_name: S, param_name_op: PersondataVorlagenParamNameOp) -> Self {
        Self { param_name: param_name.into(), param_name_op, ..self }
    }

    pub fn parameter_value<S: Into<String>>(self, param_value: S, param_value_op: PersondataVorlagenParamValueOp) -> Self {
        Self { param_value: param_value.into(), param_value_op, ..self }
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
            if self.occ_op!=PersondataVorlagenOccOp::default() {
                url += &format!("&occ_op={}", self.occ_op);
            }
        }

        if !self.param_name.is_empty() {
            url += &format!("&param={}", self.param_name);
            if self.param_name_op!=PersondataVorlagenParamNameOp::default() {
                url += &format!("&param_name_op={}", self.param_name_op);
            }
        }

        if !self.param_value.is_empty() {
            url += &format!("&value={}", self.param_value);
            if self.param_value_op!=PersondataVorlagenParamValueOp::default() {
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

    pub fn get_blocking(&self) -> Result<Vec<PersondataVorlagenResult>,ToolsError> {
        let mut ret = Vec::new();
        let url = self.generate_csv_url();
        let client = reqwest::blocking::Client::builder()
            .user_agent(TOOLS_INTERFACE_USER_AGENT)
            .build()?;
        let response = client.get(&url).send()?;
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .flexible(true)
            .from_reader(response);
        let headers = reader.headers()?.to_owned();
        for result in reader.records() {
            let record = result?;
            let entry = PersondataVorlagenResult::from_record(&headers, &record);
            ret.push(entry);
        }
        Ok(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persondata_vorlagen_query() {
        let query = PersondataVorlagenQuery::with_template("Roscher")
            .with_occurrence(4, PersondataVorlagenOccOp::Equal)
            .in_table()
            .in_template()
            .in_reference()
            .in_wikilink()
            .in_article()
            .in_comments()
            .case_sensitive()
            .parameter_name("name", PersondataVorlagenParamNameOp::Equal)
            .parameter_value("value", PersondataVorlagenParamValueOp::Equal);

        assert_eq!(query.tmpl, "Roscher");
        assert_eq!(query.with_wl, true);
        assert_eq!(query.occ, Some(4));
        assert_eq!(query.occ_op, PersondataVorlagenOccOp::Equal);
        assert_eq!(query.in_t, true);
        assert_eq!(query.in_v, true);
        assert_eq!(query.in_r, true);
        assert_eq!(query.in_l, true);
        assert_eq!(query.in_a, true);
        assert_eq!(query.in_c, true);
        assert_eq!(query.case, true);
        assert_eq!(query.param_name, "name");
        assert_eq!(query.param_name_op, PersondataVorlagenParamNameOp::Equal);
        assert_eq!(query.param_value, "value");
        assert_eq!(query.param_value_op, PersondataVorlagenParamValueOp::Equal);
    }

    #[test]
    fn test_example() {
        let query = PersondataVorlagenQuery::with_template("Roscher")
            // .parameter_name("4", PersondataVorlagenParamNameOp::default())
            ;
        let x = query.get_blocking().unwrap();
        println!("{:?}", x);
        assert!(x.len()>2000);

    }
}
