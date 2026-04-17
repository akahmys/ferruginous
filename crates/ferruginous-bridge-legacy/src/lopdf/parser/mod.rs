use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1, space0, none_of},
    combinator::map,
    multi::many0,
    sequence::{delimited, tuple},
    number::complete::float,
    IResult,
};
use bytes::Bytes;
use super::object::{Dictionary, StringFormat, ObjectId};

pub mod lexical;

pub struct Parser;

impl Parser {
    pub fn parse_object_id(input: &[u8]) -> IResult<&[u8], (u32, u16)> {
        let (input, _) = lexical::skip(input)?;
        let (input, (id_bytes, _, gen_bytes, _, _)) = tuple((
            digit1,
            space0,
            digit1,
            space0,
            tag("obj"),
        ))(input)?;

        let id = std::str::from_utf8(id_bytes)
            .map_err(|_| nom::Err::Error(nom::error::Error::new(id_bytes, nom::error::ErrorKind::MapRes)))?
            .parse::<u32>()
            .map_err(|_| nom::Err::Error(nom::error::Error::new(id_bytes, nom::error::ErrorKind::MapRes)))?;

        let gen = std::str::from_utf8(gen_bytes)
            .map_err(|_| nom::Err::Error(nom::error::Error::new(gen_bytes, nom::error::ErrorKind::MapRes)))?
            .parse::<u16>()
            .map_err(|_| nom::Err::Error(nom::error::Error::new(gen_bytes, nom::error::ErrorKind::MapRes)))?;

        Ok((input, (id, gen)))
    }

    pub fn parse_boolean(input: &[u8]) -> IResult<&[u8], bool> {
        let (input, _) = lexical::skip(input)?;
        lexical::boolean(input)
    }

    pub fn parse_null(input: &[u8]) -> IResult<&[u8], ()> {
        let (input, _) = lexical::skip(input)?;
        lexical::null(input)
    }

    pub fn parse_number(input: &[u8]) -> IResult<&[u8], super::Object> {
        let (input, _) = lexical::skip(input)?;
        let (input, val) = float::<&[u8], nom::error::Error<&[u8]>>(input)?;
        // If it can be an integer, store as Integer
        if val == val.trunc() {
            Ok((input, super::Object::Integer(val as i64)))
        } else {
            Ok((input, super::Object::Real(val as f64)))
        }
    }

    pub fn parse_name(input: &[u8]) -> IResult<&[u8], Bytes> {
        let (input, _) = lexical::skip(input)?;
        let (input, name) = lexical::name(input)?;
        Ok((input, Bytes::copy_from_slice(&name)))
    }

    pub fn parse_string(input: &[u8]) -> IResult<&[u8], (Bytes, StringFormat)> {
        let (input, _) = lexical::skip(input)?;
        alt((
            // Literal string: (...)
            map(delimited(char('('), many0(none_of(")")), char(')')), |v: Vec<char>| {
                let bytes: Vec<u8> = v.into_iter().map(|c| c as u8).collect();
                (Bytes::from(bytes), StringFormat::Literal)
            }),
            // Hex string: <...>
            map(delimited(char('<'), many0(none_of(">")), char('>')), |v: Vec<char>| {
                let bytes: Vec<u8> = v.into_iter().map(|c| c as u8).collect();
                (Bytes::from(bytes), StringFormat::Hexadecimal)
            }),
        ))(input)
    }

    pub fn parse_array(input: &[u8]) -> IResult<&[u8], Vec<super::Object>> {
        let (input, _) = lexical::skip(input)?;
        delimited(
            char('['),
            many0(|i| Self::parse_object(i)),
            char(']'),
        )(input)
    }

    pub fn parse_dictionary(input: &[u8]) -> IResult<&[u8], Dictionary> {
        let (input, _) = lexical::skip(input)?;
        let (input, pairs) = delimited(
            tag("<<"),
            many0(tuple((
                |i| Self::parse_name(i),
                |i| Self::parse_object(i),
            ))),
            tuple((lexical::skip, tag(">>"))),
        )(input)?;
        Ok((input, pairs.into_iter().collect()))
    }

    pub fn parse_stream(input: &[u8]) -> IResult<&[u8], (Dictionary, Vec<u8>)> {
        let (input, dict) = Self::parse_dictionary(input)?;
        let (input, _) = lexical::skip(input)?;
        let (input, _) = tag("stream")(input)?;
        // According to Clause 7.3.8.1, the 'stream' keyword is followed by CRLF or LF.
        let (input, _) = alt((tag("\r\n"), tag("\n")))(input)?;
        
        // Find 'endstream'. In a robust parser, we'd use the /Length entry in 'dict'.
        // For legacy support, we search for the 'endstream' tag as a fallback.
        let length = if let Some(super::Object::Integer(l)) = dict.get(b"Length".as_slice()) {
            *l as usize
        } else {
            0 // Fallback search
        };

        if length > 0 && input.len() >= length {
            let data = input[..length].to_vec();
            let rest = &input[length..];
            let (rest, _) = lexical::skip(rest)?;
            let (rest, _) = tag("endstream")(rest)?;
            Ok((rest, (dict, data)))
        } else {
            // Primitive search for endstream
            let end_tag = b"endstream";
            let pos = input.windows(end_tag.len()).position(|window| window == end_tag);
            if let Some(p) = pos {
                let data = input[..p].to_vec();
                let rest = &input[p + end_tag.len()..];
                Ok((rest, (dict, data)))
            } else {
                Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)))
            }
        }
    }

    pub fn parse_reference(input: &[u8]) -> IResult<&[u8], ObjectId> {
        let (input, _) = lexical::skip(input)?;
        let (input, (id_bytes, _, gen_bytes, _, _)) = tuple((
            digit1,
            char(' '),
            digit1,
            char(' '),
            char('R'),
        ))(input)?;

        let id = std::str::from_utf8(id_bytes)
            .map_err(|_| nom::Err::Error(nom::error::Error::new(id_bytes, nom::error::ErrorKind::MapRes)))?
            .parse::<u32>()
            .map_err(|_| nom::Err::Error(nom::error::Error::new(id_bytes, nom::error::ErrorKind::MapRes)))?;

        let gen = std::str::from_utf8(gen_bytes)
            .map_err(|_| nom::Err::Error(nom::error::Error::new(gen_bytes, nom::error::ErrorKind::MapRes)))?
            .parse::<u16>()
            .map_err(|_| nom::Err::Error(nom::error::Error::new(gen_bytes, nom::error::ErrorKind::MapRes)))?;
        Ok((input, ObjectId { id, gen }))
    }

    pub fn parse_object(input: &[u8]) -> IResult<&[u8], super::Object> {
        let (input, _) = lexical::skip(input)?;
        alt((
            // Dictionary and Array must come before String because '<<' and '<' overlap
            map(|i| Self::parse_dictionary(i), super::Object::Dictionary),
            map(|i| Self::parse_array(i), super::Object::Array),
            // Reference must come before number because they both start with digits
            map(|i| Self::parse_reference(i), super::Object::Reference),
            map(|i| Self::parse_boolean(i), super::Object::Boolean),
            map(|i| Self::parse_null(i), |_| super::Object::Null),
            |i| Self::parse_number(i),
            map(|i| Self::parse_name(i), super::Object::Name),
            map(|i| Self::parse_string(i), |(s, f)| super::Object::String(s, f)),
        ))(input)
    }
}

// In a real implementation, this would contain a full nom-based parser
// for all PDF object types, handling legacy "dirty" cases.
