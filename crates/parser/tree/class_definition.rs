use crate::ns::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDefinition {
    pub location: Location,
    pub asdoc: Option<Rc<Asdoc>>,
    pub attributes: Vec<Attribute>,
    pub name: (String, Location),
    pub type_parameters: Option<Vec<Rc<TypeParameter>>>,
    pub extends_clause: Option<Rc<Expression>>,
    pub implements_clause: Option<Vec<Rc<Expression>>>,
    pub block: Rc<Block>,
}
