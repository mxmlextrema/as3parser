use crate::ns::*;
use serde::{Serialize, Deserialize};

/// The `o.<...>` expression used for specifying the types for a parameterized type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyTypeExpression {
    pub location: Location,
    pub base: Rc<Expression>,
    pub arguments: Vec<Rc<Expression>>,
}