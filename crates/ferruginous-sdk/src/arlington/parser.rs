//! Arlington Expression AST & Parser
//!
//! (ISO 32000-2:2020 Clause 7.1 Arlington PDF Model)

use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha1, alphanumeric1, char, digit1, multispace0, none_of},
    combinator::{map, map_res, opt, recognize},
    multi::{many0, separated_list0},
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

/// Represents a compiled Arlington PDF Model expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Reference to a key in the current dictionary (e.g., @Type)
    Key(String),
    /// Reference to a key in the parent dictionary (e.g., parent::Type)
    ParentKey(String),
    /// Reference to a key in the trailer dictionary (e.g., trailer::Root)
    TrailerKey(String),
    /// Boolean literal (true/false)
    Boolean(bool),
    /// Integer literal
    Integer(i64),
    /// Real (float) literal
    Real(f64),
    /// String literal
    String(String),
    /// Function call (e.g., fn:SinceVersion(2.0, @Key))
    Function(String, Vec<Expression>),
    /// Logical AND (&&)
    And(Box<Expression>, Box<Expression>),
    /// Logical OR (||)
    Or(Box<Expression>, Box<Expression>),
    /// Equality comparison (==)
    Eq(Box<Expression>, Box<Expression>),
    /// Inequality comparison (!=)
    Ne(Box<Expression>, Box<Expression>),
    /// Greater than or equal (>=)
    Ge(Box<Expression>, Box<Expression>),
    /// Fewer than or equal (<=)
    Le(Box<Expression>, Box<Expression>),
    /// Greater than (>)
    Gt(Box<Expression>, Box<Expression>),
    /// Fewer than (<)
    Lt(Box<Expression>, Box<Expression>),
}

/// Parses an Arlington expression from a string.
pub fn parse_expression(input: &str) -> IResult<&str, Expression> {
    parse_logical_or(input)
}

fn parse_logical_or(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_logical_and(input)?;
    let (input, remainder) = many0(pair(
        preceded(multispace0, tag("||")),
        preceded(multispace0, parse_logical_and),
    ))(input)?;

    let mut res = left;
    for (_, right) in remainder {
        res = Expression::Or(Box::new(res), Box::new(right));
    }
    Ok((input, res))
}

fn parse_logical_and(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_comparison(input)?;
    let (input, remainder) = many0(pair(
        preceded(multispace0, tag("&&")),
        preceded(multispace0, parse_comparison),
    ))(input)?;

    let mut res = left;
    for (_, right) in remainder {
        res = Expression::And(Box::new(res), Box::new(right));
    }
    Ok((input, res))
}

fn parse_comparison(input: &str) -> IResult<&str, Expression> {
    let (input, left) = parse_term(input)?;
    let (input, remainder) = opt(pair(
        preceded(multispace0, alt((tag("=="), tag("!="), tag(">="), tag("<="), tag(">"), tag("<")))),
        preceded(multispace0, parse_term),
    ))(input)?;

    if let Some((op, right)) = remainder {
        let res = match op {
            "==" => Expression::Eq(Box::new(left), Box::new(right)),
            "!=" => Expression::Ne(Box::new(left), Box::new(right)),
            ">=" => Expression::Ge(Box::new(left), Box::new(right)),
            "<=" => Expression::Le(Box::new(left), Box::new(right)),
            ">" => Expression::Gt(Box::new(left), Box::new(right)),
            "<" => Expression::Lt(Box::new(left), Box::new(right)),
            _ => unreachable!(),
        };
        Ok((input, res))
    } else {
        Ok((input, left))
    }
}

fn parse_term(input: &str) -> IResult<&str, Expression> {
    preceded(
        multispace0,
        alt((
            parse_parentheses,
            parse_function_call,
            parse_reference,
            parse_literal,
        )),
    )(input)
}

fn parse_parentheses(input: &str) -> IResult<&str, Expression> {
    delimited(
        char('('),
        delimited(multispace0, parse_expression, multispace0),
        char(')'),
    )(input)
}

fn parse_function_call(input: &str) -> IResult<&str, Expression> {
    let (input, _) = tag("fn:")(input)?;
    let (input, name) = recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_"))))))(input)?;
    let (input, args) = delimited(
        char('('),
        separated_list0(
            delimited(multispace0, char(','), multispace0),
            parse_expression,
        ),
        char(')'),
    )(input)?;
    Ok((input, Expression::Function(name.to_string(), args)))
}

fn parse_reference(input: &str) -> IResult<&str, Expression> {
    alt((
        map(preceded(tag("@"), take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '*' || c == '.')), |s: &str| Expression::Key(s.to_string())),
        map(preceded(tag("parent::"), take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '*')), |s: &str| Expression::ParentKey(s.to_string())),
        map(preceded(tag("trailer::"), take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '*')), |s: &str| Expression::TrailerKey(s.to_string())),
    ))(input)
}

fn parse_literal(input: &str) -> IResult<&str, Expression> {
    alt((
        parse_boolean,
        parse_real,
        parse_integer,
        parse_string_literal,
    ))(input)
}

fn parse_boolean(input: &str) -> IResult<&str, Expression> {
    alt((
        map(tag("true"), |_| Expression::Boolean(true)),
        map(tag("false"), |_| Expression::Boolean(false)),
    ))(input)
}

fn parse_integer(input: &str) -> IResult<&str, Expression> {
    map_res(recognize(pair(opt(tag("-")), digit1)), |s: &str| s.parse().map(Expression::Integer))(input)
}

fn parse_real(input: &str) -> IResult<&str, Expression> {
    map_res(
        recognize(tuple((opt(tag("-")), digit1, char('.'), digit1))),
        |s: &str| s.parse().map(Expression::Real),
    )(input)
}

fn parse_string_literal(input: &str) -> IResult<&str, Expression> {
    map(
        delimited(char('\''), many0(none_of("'")), char('\'')),
        |v: Vec<char>| Expression::String(v.into_iter().collect()),
    )(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_key() {
        let (_, expr) = parse_expression("@Type == 'Catalog'").unwrap();
        assert!(matches!(expr, Expression::Eq(_, _)));
    }

    #[test]
    fn test_parse_function() {
        let (_, expr) = parse_expression("fn:SinceVersion(2.0, @Key)").unwrap();
        if let Expression::Function(name, args) = expr {
            assert_eq!(name, "SinceVersion");
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_logical() {
        let (_, expr) = parse_expression("@A && @B || @C").unwrap();
        assert!(matches!(expr, Expression::Or(_, _)));
    }
}
