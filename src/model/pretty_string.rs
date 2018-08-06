use std::collections::HashMap;
use game_engine::prelude::*;
use lazy_static::lazy_static;

lazy_static! {
    static ref RULES: HashMap<&'static str, Attributes> = {
        let mut map = HashMap::new();
        map.insert("blue", Attributes { font: None, color: Some(Color::BLUE) });
        map
    };
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Debug)]
pub struct Attributes {
    pub font: Option<&'static Font>,
    pub color: Option<Color>,
}

impl Attributes {
    pub fn override_with(self, other: &Attributes) -> Attributes {
        Attributes {
            font: other.font.or(self.font),
            color: other.color.or(self.color),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Default, Debug)]
pub struct PrettyString(pub Vec<(String, Attributes)>);

impl PrettyString {
    pub fn parse(string: &str) -> Self{
        parser::parse(string)
    }

    pub fn len(&self) -> usize {
        self.0
            .iter()
            .map(|(string, _)| string.len())
            .fold(0, |a, b| a + b)
    }
}

mod parser {
    use super::{PrettyString, Attributes, RULES};
    use std::ops::{Generator, GeneratorState};

    enum State {
        Start,
        Open(String),
        DoubleOpen,
        Close,
        OpenClose,
    }

    pub enum Token {
        StartSegment(String),
        EndSegment,
        Text(String),
    }

    fn lmm(text: &str, state: State) -> (Token, &str) {
        let rest = &text[1..];
        use self::State::*;
        match (state, text.chars().nth(0).unwrap()) {
            (Start, '<')                            => lmm(rest, Open("".to_owned())),
            (Start, '>')                            => lmm(rest, Close),
            (Start, ch)                             => (Token::Text(ch.to_string()), rest),
            (Open(ref st), '<') if st.is_empty()    => lmm(rest, DoubleOpen),
            (Open(ref st), '>') if st.is_empty()    => lmm(rest, OpenClose),
            (Open(ref st), ':') if st.is_empty()    => panic!("Missing rule name in dialog string"),
            (Open(st), ':')                         => (Token::StartSegment(st), rest),
            (Open(st), ch)                          => lmm(rest, Open(st + &ch.to_string())),
            (Close, _)                              => (Token::EndSegment, text),
            (DoubleOpen, _)                         => (Token::Text("<".to_owned()), text),
            (OpenClose, _)                          => (Token::Text(">".to_owned()), text),
        }
    }

    fn tokenize(mut string: &'a str) -> impl Iterator<Item = Token> + 'a {
        let gen = move || {
            while !string.is_empty() {
                let (token, rest) = lmm(string, State::Start);
                yield token;
                string = rest;
            }
        };
        generator_to_iterator(gen)
    }

    fn resolve_rules(rules: &[String]) -> Attributes {
        rules.iter()
            .fold(
                Attributes::default(),
                |attrs, rule| attrs.override_with(
                    RULES
                        .get(rule.as_str())
                        .expect(&format!("Missing rule {} in dialog string", rule))
                )
            )
    }

    pub(super) fn parse(string: &str) -> PrettyString {
        let mut segments = vec![];
        let mut rules = vec![];
        let mut segment = String::new();
        for token in tokenize(string) {
            match token {
                Token::StartSegment(name) => {
                    if !segment.is_empty() {
                        segments.push((segment, resolve_rules(&rules)));
                        segment = String::new();
                    }
                    rules.push(name);
                }
                Token::EndSegment => {
                    if !segment.is_empty() {
                        segments.push((segment, resolve_rules(&rules)));
                        segment = String::new();
                    }
                    rules.pop().expect("Ran out of rules to pop");
                }
                Token::Text(text) => segment = segment + &text,
            }
        }
        if !segment.is_empty() {
            segments.push((segment, resolve_rules(&rules)));
        }
        PrettyString(segments)
    }

    fn generator_to_iterator<G>(g: G) -> impl Iterator<Item = G::Yield>
    where G: Generator<Return = ()> {
        struct It<G>(G);

        impl<G: Generator<Return = ()>> Iterator for It<G> {
            type Item = G::Yield;

            fn next(&mut self) -> Option<Self::Item> {
                unsafe {
                    match self.0.resume() {
                        GeneratorState::Yielded(y) => Some(y),
                        GeneratorState::Complete(()) => None,
                    }
                }
            }
        }

        It(g)
    }
}
