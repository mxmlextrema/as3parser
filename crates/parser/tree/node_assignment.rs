use crate::ns::*;
use by_address::ByAddress;

const LARGE_BYTES: usize = 26_000;

/// Represents the mapping of any node to something.
/// 
/// A limited subtype of nodes may be mapped to something within this
/// structure through using the implemented `NodeAssignmentMethod`
/// methods, such as `.get()` and `.set()`.
pub struct NodeAssignment<S> {
    common: NodeAssignment1<S>,
    large_units: RefCell<HashMap<ByAddress<Rc<CompilationUnit>>, NodeAssignment1<S>>>,
}

impl<S: Clone> NodeAssignment<S> {
    pub fn new() -> Self {
        Self {
            common: NodeAssignment1::new(),
            large_units: RefCell::new(HashMap::new()),
        }
    }

    pub fn clear(&self) {
        self.common.clear();
        self.large_units.borrow_mut().clear();
    }
}

/// Defines access methods for the `NodeAssignment` structure,
/// used for attaching semantics to the syntactic tree,
/// where `T` is the node type, and `S` is the symbol type.
pub trait NodeAssignmentMethod<T, S: Clone> {
    fn get(&self, node: &Rc<T>) -> Option<S>;
    fn set(&self, node: &Rc<T>, symbol: Option<S>);
    fn delete(&self, node: &Rc<T>) -> bool;
    fn has(&self, node: &Rc<T>) -> bool;
}

macro impl_semantics_with_loc_call {
    (struct $node_assignment_id:ident, $($nodetype:ident),*$(,)?) => {
        $(
            impl<S: Clone> NodeAssignmentMethod<$nodetype, S> for $node_assignment_id<S> {
                fn get(&self, node: &Rc<$nodetype>) -> Option<S> {
                    let cu = node.location().compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.get(node)
                    } else {
                        let large_units = self.large_units.borrow();
                        let m1 = large_units.get(&ByAddress(cu));
                        m1.and_then(|m1| m1.get(node))
                    }
                }
                fn set(&self, node: &Rc<$nodetype>, symbol: Option<S>) {
                    let cu = node.location().compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.set(node, symbol);
                    } else {
                        let mut large_units = self.large_units.borrow_mut();
                        let m1 = large_units.get_mut(&ByAddress(cu.clone()));
                        if let Some(m1) = m1 {
                            m1.set(node, symbol);
                        } else {
                            let m1 = NodeAssignment1::new();
                            m1.set(node, symbol);
                            large_units.insert(ByAddress(cu), m1);
                        }
                    }
                }
                fn delete(&self, node: &Rc<$nodetype>) -> bool {
                    let cu = node.location().compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.delete(node)
                    } else {
                        let mut large_units = self.large_units.borrow_mut();
                        let m1 = large_units.get_mut(&ByAddress(cu));
                        m1.map(|m1| m1.delete(node)).unwrap_or(false)
                    }
                }
                fn has(&self, node: &Rc<$nodetype>) -> bool {
                    let cu = node.location().compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.has(node)
                    } else {
                        let large_units = self.large_units.borrow();
                        let m1 = large_units.get(&ByAddress(cu));
                        m1.map(|m1| m1.has(node)).unwrap_or(false)
                    }
                }
            }
        )*
    },
}

macro impl_semantics_with_loc_field {
    (struct $node_assignment_id:ident, $($nodetype:ident),*$(,)?) => {
        $(
            impl<S: Clone> NodeAssignmentMethod<$nodetype, S> for $node_assignment_id<S> {
                fn get(&self, node: &Rc<$nodetype>) -> Option<S> {
                    let cu = node.location.compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.get(node)
                    } else {
                        let large_units = self.large_units.borrow();
                        let m1 = large_units.get(&ByAddress(cu));
                        m1.and_then(|m1| m1.get(node))
                    }
                }
                fn set(&self, node: &Rc<$nodetype>, symbol: Option<S>) {
                    let cu = node.location.compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.set(node, symbol);
                    } else {
                        let mut large_units = self.large_units.borrow_mut();
                        let m1 = large_units.get_mut(&ByAddress(cu.clone()));
                        if let Some(m1) = m1 {
                            m1.set(node, symbol);
                        } else {
                            let m1 = NodeAssignment1::new();
                            m1.set(node, symbol);
                            large_units.insert(ByAddress(cu), m1);
                        }
                    }
                }
                fn delete(&self, node: &Rc<$nodetype>) -> bool {
                    let cu = node.location.compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.delete(node)
                    } else {
                        let mut large_units = self.large_units.borrow_mut();
                        let m1 = large_units.get_mut(&ByAddress(cu));
                        m1.map(|m1| m1.delete(node)).unwrap_or(false)
                    }
                }
                fn has(&self, node: &Rc<$nodetype>) -> bool {
                    let cu = node.location.compilation_unit();
                    if cu.text().len() < LARGE_BYTES {
                        self.common.has(node)
                    } else {
                        let large_units = self.large_units.borrow();
                        let m1 = large_units.get(&ByAddress(cu));
                        m1.map(|m1| m1.has(node)).unwrap_or(false)
                    }
                }
            }
        )*
    },
}

macro impl_semantics_1 {
    (struct $node_assignment_1_id:ident, fn $new_id:ident, fn $clear_id:ident, $($nodetype:ident),*$(,)?) => {
        #[allow(non_snake_case)]
        struct $node_assignment_1_id<S> {
            $($nodetype: RefCell<HashMap<NodeAsKey<Rc<$nodetype>>, Option<S>>>,)*
        }

        impl<S: Clone> $node_assignment_1_id<S> {
            pub fn $new_id() -> Self {
                Self {
                    $($nodetype: RefCell::new(HashMap::new()),)*
                }
            }

            pub fn $clear_id(&self) {
                $(self.$nodetype.borrow_mut().clear();)*
            } 
        }

        $(
            impl<S: Clone> NodeAssignmentMethod<$nodetype, S> for $node_assignment_1_id<S> {
                fn get(&self, node: &Rc<$nodetype>) -> Option<S> {
                    self.$nodetype.borrow().get(&NodeAsKey(node.clone())).map(|v| v.clone().unwrap())
                }
                fn set(&self, node: &Rc<$nodetype>, symbol: Option<S>) {
                    self.$nodetype.borrow_mut().insert(NodeAsKey(node.clone()), symbol);
                }
                fn delete(&self, node: &Rc<$nodetype>) -> bool {
                    self.$nodetype.borrow_mut().remove(&NodeAsKey(node.clone())).is_some()
                }
                fn has(&self, node: &Rc<$nodetype>) -> bool {
                    self.$nodetype.borrow().contains_key(&NodeAsKey(node.clone()))
                }
            }
        )*
    },
}

impl_semantics_with_loc_call!(
    struct NodeAssignment,
    Expression,
    InitializerField,
    Directive,
    MxmlContent,
    CssDirective,
    CssMediaQueryCondition,
    CssSelectorCondition,
    CssPropertyValue,
    CssSelector,
);

impl_semantics_with_loc_field!(
    struct NodeAssignment,
    FunctionCommon,
    Block,
    Program,
    PackageDefinition,
    SimpleVariableDefinition,
    Metadata,
    MetadataEntry,
    Mxml,
    MxmlElement,
    MxmlAttribute,
    CssProperty,
    CssRule,
    CssDocument,
    QualifiedIdentifier,
);

impl_semantics_1!(
    struct NodeAssignment1,
    fn new,
    fn clear,
    Expression,
    InitializerField,
    Directive,
    FunctionCommon,
    Block,
    Program,
    PackageDefinition,
    SimpleVariableDefinition,
    QualifiedIdentifier,
    Metadata,
    MetadataEntry,
    Mxml,
    MxmlContent,
    MxmlElement,
    MxmlAttribute,
    CssDirective,
    CssRule,
    CssMediaQueryCondition,
    CssSelectorCondition,
    CssPropertyValue,
    CssSelector,
    CssProperty,
    CssDocument,
);