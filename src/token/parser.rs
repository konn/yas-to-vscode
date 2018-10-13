use super::*;
use combine::parser::char::*;
use combine::*;
use std::marker::PhantomData;

#[derive(Debug)]
pub struct TokenParser<'a> {
    proxy: PhantomData<&'a ()>,
    pub warnings: Vec<Warning>,
}

fn number<'a>(input: &mut &'a str) -> ParseResult<usize, &'a str> {
    many1::<String, _>(digit())
        .map(|a| a.parse::<usize>().unwrap())
        .parse_stream(input)
}

fn string_lit<'a>(input: &mut &'a str) -> ParseResult<String, &'a str> {
    token('"')
        .with(many(none_of(vec!['"'])))
        .skip(token('"'))
        .parse_stream(input)
}

fn symbol<'a>(sym: &'static str) -> impl Parser<Input = &'a str, Output = ()> {
    spaces().skip(string(sym))
}

fn reserved<'a>(sym: &'static str) -> impl Parser<Input = &'a str, Output = ()> {
    spaces()
        .skip(string(sym))
        .skip(not_followed_by(alpha_num()))
}

impl<'a> TokenParser<'a> {
    pub fn new() -> TokenParser<'a> {
        TokenParser {
            proxy: PhantomData,
            warnings: vec![],
        }
    }
}

pub fn snippet<'a>(input: &mut &'a str) -> ParseResult<Token, &'a str> {
    parser(|a| lines(false, a)).skip(eof()).parse_stream(input)
}

fn raw<'a>(inside: bool, input: &mut &'a str) -> ParseResult<Token, &'a str> {
    let banned = if inside { "}\r\n$" } else { "\r\n$" };
    let mut p = attempt(
        token('$')
            .map(|_| Raw("$".to_string()))
            .skip(not_followed_by(token('{').or(digit()))),
    ).or(many1::<String, _>(none_of(banned.chars()).expected("Non-newline nor dollar")).map(Raw));
    p.parse_stream(input)
}

fn lines<'a>(inside: bool, input: &mut &'a str) -> ParseResult<Token, &'a str> {
    let mut p = sep_end_by(parser(|a| line(inside, a)), newline()).map(|ls: Vec<_>| {
        if ls.len() == 1 {
            ls.into_iter().next().unwrap()
        } else {
            Lines(ls)
        }
    });
    p.parse_stream(input)
}

fn line<'a>(inside: bool, input: &mut &'a str) -> ParseResult<Token, &'a str> {
    many(parser(move |i| raw(inside, i)).or(parser(primitive)))
        .map(|ts: Vec<_>| {
            if ts.len() == 1 {
                ts.into_iter().next().unwrap()
            } else {
                Inline(ts)
            }
        }).parse_stream(input)
}

fn primitive<'a>(input: &mut &'a str) -> ParseResult<Token, &'a str> {
    let mut p = attempt(parser(choice))
        .expected("Choice statement")
        .or(parser(tabstop).message("$ must be followed by a tabstop/placeholder"));
    p.parse_stream(input)
}

fn choice<'a>(input: &mut &'a str) -> ParseResult<Token, &'a str> {
    let mut p = string("${")
        .with(
            parser(number)
                .skip(symbol(":"))
                .skip(string("$$("))
                .skip(reserved("yas-choose-value"))
                .skip(symbol("'("))
                .skip(spaces())
                .then(move |n| {
                    sep_end_by(parser(string_lit), spaces()).map(move |alts| Choice {
                        number: n,
                        alternatives: alts,
                    })
                }).skip(token(')'))
                .skip(symbol(")")),
        ).skip(symbol("}"));
    p.parse_stream(input)
}

fn tabstop<'a>(input: &mut &'a str) -> ParseResult<Token, &'a str> {
    let complex = token('{')
        .with(parser(number).expected("Tabstop number").then(|n| {
            symbol(":")
                .expected("Colon")
                .with(parser(|a| lines(true, a)).message("Some line(s) expected after `:'"))
                .map(move |tks| Tabstop {
                    number: n,
                    contents: Some(Box::new(tks)),
                })
        })).skip(symbol("}").expected("Closing `}'"));
    let simple = parser(number).map(|s| Tabstop {
        number: s,
        contents: None,
    });
    let mut parser = token('$').with(complex.or(simple));

    parser.parse_stream(input)
}

impl<'a> Parser for TokenParser<'a> {
    type Input = &'a str;
    type Output = Token;
    type PartialState = ();

    fn parse_stream(&mut self, input: &mut &'a str) -> ParseResult<Token, &'a str> {
        parser(snippet).parse_stream(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strlit_parse() {
        assert_eq!(
            parser(string_lit).parse("\"foo\"").map(|x| x.0),
            Ok("foo".to_string())
        );
    }

    #[test]
    fn choice_parse() {
        let alts = vec!["foo", "bar", "buz"]
            .into_iter()
            .map(|a| a.to_string())
            .collect();
        let src = "${25:$$(yas-choose-value '( \n\t\"foo\"\"bar\" \n  \"buz\" ))}";
        assert_eq!(
            parser(choice).parse(src).map(|x| x.0),
            Ok(Choice {
                number: 25,
                alternatives: alts
            })
        );
    }

    #[test]
    fn snippet_parse() {
        let src = r#"type role  ${1:Type} ${2:$$(
                    yas-choose-value
                    '("representational""phantom"  
                    "nominal" "_"))}"#;
        let roles = vec!["representational", "phantom", "nominal", "_"]
            .into_iter()
            .map(|a| a.to_string())
            .collect();
        assert_eq!(
            parser(snippet).parse(src).map(|x| x.0),
            Ok(Inline(vec![
                Raw("type role  ".to_string()),
                Tabstop {
                    number: 1,
                    contents: Some(Box::new(Raw("Type".to_string())))
                },
                Raw(" ".to_string()),
                Choice {
                    number: 2,
                    alternatives: roles
                }
            ]))
        );

        let src = r#"impl${1:<${2:T}>} ${3:Type$1} {
    ${4:fn ${5:new}(${6:arg}) -> ${7:$3} {
        ${8:$3 { ${9:arg} } }
    }
    }$0
}"#;
        assert_eq!(
            parser(snippet).parse(src).map(|x| x.0),
            Ok(Lines(vec![
                Inline(vec![
                    Raw("impl".to_string()),
                    Tabstop {
                        number: 1,
                        contents: Some(Box::new(Inline(vec![
                            Raw("<".to_string()),
                            Tabstop {
                                number: 2,
                                contents: Some(Box::new(Raw("T".to_string())))
                            },
                            Raw(">".to_string())
                        ])))
                    },
                    Raw(" ".to_string()),
                    Tabstop {
                        number: 3,
                        contents: Some(Box::new(Inline(vec![
                            Raw("Type".to_string()),
                            Tabstop {
                                number: 1,
                                contents: None
                            }
                        ])))
                    },
                    Raw(" {".to_string())
                ]),
                Inline(vec![
                    Raw("    ".to_string()),
                    Tabstop {
                        number: 4,
                        contents: Some(Box::new(Lines(vec![
                            Inline(vec![
                                Raw("fn ".to_string()),
                                Tabstop {
                                    number: 5,
                                    contents: Some(Box::new(Raw("new".to_string())))
                                },
                                Raw("(".to_string()),
                                Tabstop {
                                    number: 6,
                                    contents: Some(Box::new(Raw("arg".to_string())))
                                },
                                Raw(") -> ".to_string()),
                                Tabstop {
                                    number: 7,
                                    contents: Some(Box::new(Tabstop {
                                        number: 3,
                                        contents: None
                                    }))
                                },
                                Raw(" {".to_string())
                            ]),
                            Inline(vec![
                                Raw("        ".to_string()),
                                Tabstop {
                                    number: 8,
                                    contents: Some(Box::new(Inline(vec![
                                        Tabstop {
                                            number: 3,
                                            contents: None
                                        },
                                        Raw(" { ".to_string()),
                                        Tabstop {
                                            number: 9,
                                            contents: Some(Box::new(Raw("arg".to_string())))
                                        },
                                        Raw(" ".to_string())
                                    ])))
                                },
                                Raw(" ".to_string())
                            ])
                        ])))
                    }
                ]),
                Raw("    }".to_string()),
                Inline(vec![
                    Raw("    }".to_string()),
                    Tabstop {
                        number: 0,
                        contents: None
                    }
                ]),
                Raw("}".to_string())
            ]))
        )
    }
}
