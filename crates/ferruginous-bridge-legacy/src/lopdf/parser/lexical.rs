use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, multispace0, none_of},
    combinator::map,
    multi::many0,
    IResult,
};

/// PDF White-space characters (Clause 7.2.2).
pub fn whitespace(input: &[u8]) -> IResult<&[u8], &[u8]> {
    multispace0(input)
}

/// PDF Comments (Clause 7.2.3).
pub fn comment(input: &[u8]) -> IResult<&[u8], ()> {
    let (input, _) = char('%')(input)?;
    let (input, _) = many0(none_of("\r\n"))(input)?;
    let (input, _) = alt((tag("\r\n"), tag("\r"), tag("\n"), tag("")))(input)?;
    Ok((input, ()))
}

/// Skip whitespace and comments.
pub fn skip(input: &[u8]) -> IResult<&[u8], ()> {
    let (mut input, _) = whitespace(input)?;
    while let Ok((next_input, _)) = comment(input) {
        let (next_input, _) = whitespace(next_input)?;
        input = next_input;
    }
    Ok((input, ()))
}

/// PDF Boolean (Clause 7.3.2).
pub fn boolean(input: &[u8]) -> IResult<&[u8], bool> {
    alt((
        map(tag("true"), |_| true),
        map(tag("false"), |_| false),
    ))(input)
}

/// PDF Null (Clause 7.3.9).
pub fn null(input: &[u8]) -> IResult<&[u8], ()> {
    map(tag("null"), |_| ())(input)
}

/// PDF Name (Clause 7.3.5).
pub fn name(input: &[u8]) -> IResult<&[u8], Vec<u8>> {
    let (input, _) = char('/')(input)?;
    // A name is a sequence of non-white-space and non-delimiter characters.
    let (input, content) = many0(none_of("()<>[]{}/% \t\r\n"))(input)?;
    let bytes = content.iter().map(|&c| c as u8).collect();
    Ok((input, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip() {
        let input = b"  % comment\n   true";
        let (rest, _) = skip(input).unwrap();
        assert_eq!(rest, b"true");
    }

    #[test]
    fn test_boolean() {
        assert!(boolean(b"true").unwrap().1);
        assert!(!boolean(b"false").unwrap().1);
    }
}
