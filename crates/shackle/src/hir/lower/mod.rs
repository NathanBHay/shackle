//! Functionality for converting AST nodes to HIR nodes
//! for the respective modelling languages.

pub mod minizinc;
pub mod eprime;
pub mod test;
use std::sync::Arc;

use crate::constants::IdentifierRegistry;
use crate::file::ModelRef;
use crate::hir::db::Hir;
use crate::hir::source::SourceMap;
use crate::hir::*;
use crate::syntax::ast::ConstraintModel;
use crate::Error;

use self::minizinc::ItemCollector;
use self::eprime::ItemCollector as EPrimeItemCollector;

/// Lower a model to HIR
pub fn lower_items(db: &dyn Hir, model: ModelRef) -> (Arc<Model>, Arc<SourceMap>, Arc<Vec<Error>>) {
	let ast = match db.ast(*model) {
		Ok(m) => m,
		Err(e) => return (Default::default(), Default::default(), Arc::new(vec![e])),
	};
    let identifiers = IdentifierRegistry::new(db);
	match ast {
		ConstraintModel::MznModel(ast) => {
			let mut ctx = ItemCollector::new(db, &identifiers, model);
			for item in ast.items() {
				ctx.collect_item(item);
			}
			let (m, sm, e) = ctx.finish();
			(Arc::new(m), Arc::new(sm), Arc::new(e))
		}
		ConstraintModel::EPrimeModel(ast) => {
			let mut ctx = EPrimeItemCollector::new(db, &identifiers, model);
			for item in ast.items() {
				ctx.collect_item(item);
			}
			let (m, sm, e) = ctx.finish();
			(Arc::new(m), Arc::new(sm), Arc::new(e))
		}
	}
}