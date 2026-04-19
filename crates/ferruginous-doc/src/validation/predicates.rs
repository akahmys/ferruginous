use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    combinator::map,
    multi::separated_list0,
    sequence::{delimited, preceded, tuple},
    IResult,
};
use ferruginous_core::{Object, PdfName};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    SinceVersion(String, String),
    Required(String),
    Deprecated(String, String),
    IsRequired(Box<Predicate>, String),
    Key(String),
    Value(String),
}

pub struct PredicateEvaluator<'a> {
    pub version: &'a str,
    pub context: &'a BTreeMap<PdfName, Object>,
}

impl<'a> PredicateEvaluator<'a> {
    pub fn evaluate(&self, predicate: &Predicate) -> bool {
        match predicate {
            Predicate::SinceVersion(ver, _key) => {
                self.version >= ver.as_str()
            }
            Predicate::Required(key) => {
                self.context.contains_key(&PdfName::from(key.as_str()))
            }
            Predicate::Deprecated(ver, _key) => {
                self.version >= ver.as_str()
            }
            Predicate::IsRequired(cond, key) => {
                if self.evaluate(cond) {
                    self.context.contains_key(&PdfName::from(key.as_str()))
                } else {
                    true // Not required if condition is false
                }
            }
            Predicate::Key(key) => self.context.contains_key(&PdfName::from(key.as_str())),
            Predicate::Value(val) => val == "true", // Simplified
        }
    }
}

// Parser implementation
fn parse_fn_name(input: &str) -> IResult<&str, &str> {
    preceded(tag("fn:"), take_while1(|c: char| c.is_alphanumeric()))(input)
}

fn parse_arg(input: &str) -> IResult<&str, &str> {
    delimited(multispace0, take_while1(|c: char| c.is_alphanumeric() || c == '.'), multispace0)(input)
}

fn parse_args(input: &str) -> IResult<&str, Vec<&str>> {
    delimited(char('('), separated_list0(char(','), parse_arg), char(')'))(input)
}

pub fn parse_predicate(input: &str) -> IResult<&str, Predicate> {
    alt((
        map(tuple((parse_fn_name, parse_args)), |(name, args)| {
            match name {
                "SinceVersion" if args.len() >= 2 => Predicate::SinceVersion(args[0].to_string(), args[1].to_string()),
                "Required" if !args.is_empty() => Predicate::Required(args[0].to_string()),
                "Deprecated" if args.len() >= 2 => Predicate::Deprecated(args[0].to_string(), args[1].to_string()),
                _ => Predicate::Value(name.to_string()),
            }
        }),
        map(take_while1(|c: char| c.is_alphanumeric()), |s: &str| Predicate::Key(s.to_string())),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_since_version() {
        let input = "fn:SinceVersion(2.0,Catalog)";
        let (_, p) = parse_predicate(input).unwrap();
        assert_eq!(p, Predicate::SinceVersion("2.0".into(), "Catalog".into()));
    }

    #[test]
    fn test_evaluate_since_version() {
        let p = Predicate::SinceVersion("2.0".into(), "Metadata".into());
        let context = BTreeMap::new();
        let eval = PredicateEvaluator { version: "2.0", context: &context };
        assert!(eval.evaluate(&p));
        
        let eval_old = PredicateEvaluator { version: "1.7", context: &context };
        assert!(!eval_old.evaluate(&p));
    }
}
