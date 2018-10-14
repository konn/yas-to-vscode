use super::*;

pub fn validate(src: String) -> (Vec<String>, Vec<Warning>) {
    let (tok, warns) = Token::from_str(&src);
    (tok.render(), warns)
}

#[derive(Eq, PartialEq, Debug)]
pub enum Token {
    Lines(Vec<Token>),
    Inline(Vec<Token>),
    Tabstop {
        number: usize,
        contents: Option<Box<Token>>,
    },
    Choice {
        number: usize,
        alternatives: Vec<String>,
    },
    Raw(String),
}

use self::Token::*;
pub mod parser;
use self::parser::TokenParser;
use combine::parser::Parser;

impl Token {
    fn from_str(src: &str) -> (Token, Vec<Warning>) {
        let mut parser = TokenParser::new();
        match parser.parse(src) {
            Ok((val, _)) => (val, vec![]),
            _ => (Raw(src.to_string()), vec![Warning::SnippetBodyParseFailed]),
        }
    }

    fn render(self) -> Vec<String> {
        match self {
            Lines(tks) => tks.into_iter().flat_map(|a| a.render()).collect(),
            Inline(tks) => vec![
                tks.into_iter()
                    .map(|a| a.render().join("\n"))
                    .collect::<Vec<_>>()
                    .join(""),
            ],
            Raw(str) => vec![str],
            Choice {
                number,
                alternatives,
            } => vec![format!("${{{}|{}|}}", number, alternatives.join(","))],
            Tabstop { number, contents } => {
                let body = contents.map_or("".to_string(), |a| a.render().join("\n"));
                vec![if body.is_empty() {
                    format!("${}", number)
                } else {
                    format!("${{{}:{}}}", number, body)
                }]
            }
        }
    }
}
