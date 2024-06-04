use crate::ns::*;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchStatement {
    pub location: Location,
    pub discriminant: Rc<Expression>,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Case {
    pub location: Location,
    pub labels: Vec<CaseLabel>,
    pub directives: Vec<Rc<Directive>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaseLabel {
    Case((Rc<Expression>, Location)),
    Default(Location),
}

impl CaseLabel {
    pub fn location(&self) -> Location {
        match self {
            Self::Case((_, l)) => l.clone(),
            Self::Default(l) => l.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchTypeStatement {
    pub location: Location,
    pub discriminant: Rc<Expression>,
    pub cases: Vec<TypeCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCase {
    pub location: Location,
    /// Case parameter. If `None`, designates a `default {}` case.
    pub parameter: Option<TypedDestructuring>,
    pub block: Rc<Block>,
}