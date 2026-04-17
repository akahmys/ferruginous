//! Arlington Expression Evaluator
//!
//! (ISO 32000-2:2020 Clause 7.1 Arlington PDF Model)

use std::collections::BTreeMap;
use crate::core::{Object, Resolver};
use crate::arlington::parser::Expression;

/// Evaluation context for Arlington predicates.
pub struct EvalContext<'a> {
    /// The current object's dictionary.
    pub dictionary: &'a BTreeMap<Vec<u8>, Object>,
    /// Optional parent dictionary.
    pub parent: Option<&'a BTreeMap<Vec<u8>, Object>>,
    /// Optional trailer dictionary.
    pub trailer: Option<&'a BTreeMap<Vec<u8>, Object>>,
    /// Resolver for indirect objects.
    pub resolver: &'a dyn Resolver,
    /// PDF version of the current document (e.g., 2.0).
    pub version: f64,
}

/// Result of evaluating an Arlington expression.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalValue {
    /// A boolean value.
    Boolean(bool),
    /// An integer value.
    Integer(i64),
    /// A real (float) value.
    Real(f64),
    /// A string or name value.
    String(String),
    /// Represents no value or an undefined key.
    Null,
}

impl EvalValue {
    /// Returns the boolean representation of the value.
    /// Non-boolean values default to false.
    pub fn as_bool(&self) -> bool {
        match self {
            Self::Boolean(b) => *b,
            _ => false,
        }
    }
}

/// Evaluates an Arlington expression within a given context.
pub fn evaluate(expr: &Expression, ctx: &EvalContext) -> EvalValue {
    match expr {
        Expression::Boolean(b) => EvalValue::Boolean(*b),
        Expression::Integer(i) => EvalValue::Integer(*i),
        Expression::Real(r) => EvalValue::Real(*r),
        Expression::String(s) => EvalValue::String(s.clone()),
        
        Expression::Key(key) => lookup_key(ctx.dictionary, key, ctx.resolver),
        Expression::ParentKey(key) => ctx.parent.map_or(EvalValue::Null, |d| lookup_key(d, key, ctx.resolver)),
        Expression::TrailerKey(key) => ctx.trailer.map_or(EvalValue::Null, |d| lookup_key(d, key, ctx.resolver)),

        Expression::And(l, r) => EvalValue::Boolean(evaluate(l, ctx).as_bool() && evaluate(r, ctx).as_bool()),
        Expression::Or(l, r) => EvalValue::Boolean(evaluate(l, ctx).as_bool() || evaluate(r, ctx).as_bool()),

        Expression::Eq(l, r) => EvalValue::Boolean(evaluate(l, ctx) == evaluate(r, ctx)),
        Expression::Ne(l, r) => EvalValue::Boolean(evaluate(l, ctx) != evaluate(r, ctx)),
        
        Expression::Function(name, args) => evaluate_function(name, args, ctx),
        
        _ => EvalValue::Null, // Placeholder for other comparisons
    }
}

fn lookup_key(dict: &BTreeMap<Vec<u8>, Object>, key: &str, resolver: &dyn Resolver) -> EvalValue {
    if let Some(obj) = dict.get(key.as_bytes()) {
        let obj = resolver.resolve_if_ref(obj).unwrap_or(obj.clone());
        match obj {
            Object::Boolean(b) => EvalValue::Boolean(b),
            Object::Integer(i) => EvalValue::Integer(i),
            Object::Real(r) => EvalValue::Real(r),
            Object::String(s) | Object::Name(s) => EvalValue::String(String::from_utf8_lossy(&s).into_owned()),
            _ => EvalValue::Null,
        }
    } else {
        EvalValue::Null
    }
}

fn evaluate_function(name: &str, args: &[Expression], ctx: &EvalContext) -> EvalValue {
    match name {
        "SinceVersion" => {
            if !args.is_empty() {
                if let EvalValue::Real(v) = evaluate(&args[0], ctx) {
                    return EvalValue::Boolean(ctx.version >= v);
                }
            }
            EvalValue::Boolean(false)
        }
        "Deprecated" => {
            if !args.is_empty() {
                if let EvalValue::Real(v) = evaluate(&args[0], ctx) {
                    return EvalValue::Boolean(ctx.version >= v);
                }
            }
            EvalValue::Boolean(false)
        }
        "Required" => {
            if !args.is_empty() {
                let val = evaluate(&args[0], ctx);
                return EvalValue::Boolean(val != EvalValue::Null);
            }
            EvalValue::Boolean(false)
        }
        "IsRequired" => {
             // Logic for @Key requiredness
             EvalValue::Boolean(true)
        }
        _ => EvalValue::Null,
    }
}
