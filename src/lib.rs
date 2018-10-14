#[macro_use]
extern crate serde_derive;

extern crate combine;
extern crate serde;

use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Snippet {
    pub description: String,
    pub prefix: String,
    pub body: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Warning {
    ElispCodeFound,
    SnippetBodyParseFailed,
}

#[derive(Debug)]
pub enum Error {
    ParseError(String),
    MissingField(String),
}

#[derive(Debug)]
pub struct Converted<T> {
    pub result: T,
    pub warnings: Vec<Warning>,
}

pub type Result<T> = std::result::Result<Converted<T>, Error>;

mod token;
use token::validate;

fn separating(line: &str) -> bool {
    if &line[0..1] != "#" {
        return false;
    }
    (&line[1..]).trim().chars().all(|c| c == '-')
}

use Error::*;
impl Snippet {
    pub fn parse(name: &str, src: &str) -> Result<Snippet> {
        let lines: Vec<_> = src.lines().collect();
        let i = lines
            .iter()
            .position(|a| separating(a))
            .ok_or(ParseError("No metadata separator found".to_string()))?;
        let src = lines[i + 1..].join("\n");
        let mut dic: HashMap<_, _> = lines[0..i]
            .into_iter()
            .map(|l| {
                let mut it = (&l[1..]).split(":");
                (
                    it.next().unwrap().trim(),
                    it.collect::<Vec<_>>().join(":").trim().to_string(),
                )
            }).collect();
        let description: String = dic.remove("description").unwrap_or("".to_string());
        let prefix = dic.remove("key").unwrap_or(name.to_string());
        let (body, warnings) = validate(src);
        let result = Snippet {
            prefix,
            body,
            description,
        };
        Ok(Converted { result, warnings })
    }
}

#[cfg(test)]
mod tests {
  use super::*;
  
  #[test]
  fn check_sep() {
      assert!(separating("# --"))
  }

  #[test]
  fn check_sep_2() {
      let src = r#"# -*- mode: snippet -*-
# name: assert!
# key: ass
# --
assert!(${true});"
  }
}"#;
        assert_eq!(src.lines().position(separating), Some(3));
  }
}