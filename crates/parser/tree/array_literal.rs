use crate::ns::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArrayLiteral {
    pub location: Location,
    /// ASDoc. Always ignore this field; it is used solely
    /// when parsing meta-data.
    pub asdoc: Option<Rc<AsDoc>>,
    pub elements: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Element {
    Elision,
    Expression(Rc<Expression>),
    Rest((Rc<Expression>, Location)),
}