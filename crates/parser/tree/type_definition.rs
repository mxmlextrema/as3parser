use crate::ns::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub location: Location,
    pub asdoc: Option<Rc<Asdoc>>,
    pub attributes: Vec<Attribute>,
    pub left: (String, Location),
    pub right: Rc<Expression>,
}