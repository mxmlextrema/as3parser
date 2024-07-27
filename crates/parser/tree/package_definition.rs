use crate::ns::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDefinition {
    pub location: Location,
    pub asdoc: Option<Rc<Asdoc>>,
    pub name: Vec<(String, Location)>,
    pub block: Rc<Block>,
}