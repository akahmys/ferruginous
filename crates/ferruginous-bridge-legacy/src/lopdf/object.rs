//! Modified lopdf Object model for zero-copy Bytes.
//! 
//! Original Copyright (c) 2016-2022 J-F-Liu
//! Licensed under MIT License.

use bytes::Bytes;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Null,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Name(Bytes),
    String(Bytes, StringFormat),
    Array(Vec<Object>),
    Dictionary(Dictionary),
    Stream(Stream),
    Reference(ObjectId),
}

pub type Array = Vec<Object>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StringFormat {
    Literal,
    Hexadecimal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ObjectId {
    pub id: u32,
    pub gen: u16,
}

pub type Dictionary = BTreeMap<Bytes, Object>;

#[derive(Debug, Clone, PartialEq)]
pub struct Stream {
    pub dict: Dictionary,
    pub content: Bytes,
    /// Whether the stream is decoded.
    pub allows_compression: bool,
}

impl Object {
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Object::Integer(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&Dictionary> {
        match *self {
            Object::Dictionary(ref dict) => Some(dict),
            Object::Stream(ref stream) => Some(&stream.dict),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&[u8]> {
        match self {
            Object::String(bytes, _) => Some(bytes),
            Object::Name(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Object]> {
        match self {
            Object::Array(arr) => Some(arr),
            _ => None,
        }
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} R", self.id, self.gen)
    }
}
