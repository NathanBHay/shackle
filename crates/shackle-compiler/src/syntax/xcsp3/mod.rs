//! AST representation
//!
//! AST nodes are thin wrappers around CST nodes and provide type-safe access
//! methods. No desugaring is performed at this stage, so all language constructs
//! are available other than parentheses which are implicit in the tree structure.

use std::{fmt::Debug, marker::PhantomData};

use super::{ast::Children, cst::Cst};

/// XCSP3Model (wrapper for a CST).
///
/// A model is a single `.eprime` file.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct XCSP3Model {
	cst: Cst,
}

impl XCSP3Model {
	/// Create a model from a CST
	pub fn new(cst: Cst) -> Self {
		Self { cst }
	}

	/// Get the CST
	pub fn cst(&self) -> &Cst {
		&self.cst
	}

	/// Get the top level items in the model
	pub fn items(&self) -> Children<'_, Item> {
		let tree = &self.cst;
		let id = tree.language().field_id_for_name("item").unwrap();
		let mut cursor = tree.root_node().walk();
		let done = !cursor.goto_first_child();
		Children {
			field: id,
			tree,
			cursor,
			done,
			phantom: PhantomData,
		}
	}
}

impl Debug for XCSP3Model {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Model")
			.field("items", &self.items())
			.finish()
	}
}

// Temp Help Files

use crate::syntax::ast::AstNode;
use crate::syntax::cst::CstNode;

/// Helper to retrieve a child node by its field name
pub fn attribute_with_name<T: AstNode, U: From<CstNode>>(parent: &T, field: &str) -> U {
    let tree = parent.cst_node().cst();
    let node = parent.cst_node().as_ref();
    let attribute = node.child_by_field_name("attribute").unwrap();
    if let Some(name) = attribute.child_by_field_name("name") {
        if name.text() == field {
            U::from(attribute.child_by_field_name("value").unwrap())
        }
    }
}


/*
name -> "id"
type -> "type"
as -> "as"
size -> "size"
start index -> "startIndex"

Types:
    Default type is integer
    "integer" which is a intVal or int interval
    "Symbolic" which are enums
    "real" which are floats
    "set" which has s


Var:
    name 
    type
    as 

Array:
    name
    type
    as
    size
    start index

*/