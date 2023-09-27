use crate::constants::IdentifierRegistry;
use crate::file::ModelRef;
use crate::hir::db::Hir;
use crate::hir::lower::eprime ::ExpressionCollector;
use crate::hir::ids::ItemRef;
use crate::hir::source::{Origin, SourceMap};
use crate::hir::*;
use crate::syntax::eprime;
use crate::Error;

pub struct ItemCollector<'a> {
	db: &'a dyn Hir,
	identifiers: &'a IdentifierRegistry,
	model: Model,
	source_map: SourceMap,
	diagnostics: Vec<Error>,
	owner: ModelRef,
}

impl ItemCollector<'_> {
	/// Create a new item collector
	pub fn new<'a>(
		db: &'a dyn Hir,
		identifiers: &'a IdentifierRegistry,
		owner: ModelRef,
	) -> ItemCollector<'a> {
		ItemCollector {
			db,
			identifiers,
			model: Model::default(),
			source_map: SourceMap::default(),
			diagnostics: Vec::new(),
			owner,
		}
	}

	/// Lower an AST item to HIR
	pub fn collect_item(&mut self, item: eprime::Item) {
        let (it, sm) = match item.clone() {
			eprime::Item::Constraint(c) => return self.collect_constraint(c),
			eprime::Item::ConstDefinition(c) => self.collect_const_definition(c),
			// eprime::Item::DomainAlias(d) => self.collect_domain_alias(d),
			// eprime::Item::DecisionDeclaration(d) => ,
			eprime::Item::Objective(o) => self.collect_objective(o),
            // eprime::Item::ParamDeclaration(p) =>,
			// eprime::Item::Branching(_) => return, // TODO: Currently Supported With Annotations
			eprime::Item::Heuristic(_) => return, // Currently not supported
			_ => unimplemented!("Item not implemented"),
		};
		self.source_map.insert(it.into(), Origin::new(&item));
		self.source_map.add_from_item_data(self.db, it, &sm);
	}

	/// Finish lowering
	pub fn finish(self) -> (Model, SourceMap, Vec<Error>) {
		(self.model, self.source_map, self.diagnostics)
	}

	/// Checks if a solve item exists, if not, adds satisfy solve
	/// TODO: Check if Source Map is Needed for this
	pub fn check_solve(&mut self) {
		if self.model.solves.is_empty() {
			let index = self.model.solves.insert(Item::new(
				Solve {
					goal: Goal::Satisfy,
					annotations: Box::new([]),
				},
				ItemData::default(),
			));
			self.model.items.insert((self.model.items.len()-1).max(0), index.into());
		};
		// let it = ItemRef::new(self.db, self.owner, index);
		// self.source_map.insert(it.into(), Origin::default());
		// self.source_map.add_from_item_data(self.db, it, &ItemDataSourceMap::default());
	}

	fn collect_const_definition(&mut self, c: eprime::ConstDefinition) -> (ItemRef, ItemDataSourceMap) {
		let mut ctx = ExpressionCollector::new(self.db, self.identifiers, &mut self.diagnostics);
		let assignee = ctx.collect_expression(c.name());
		let definition = ctx.collect_expression(c.definition());
		let (data, source_map) = ctx.finish();
		let index = self.model.assignments.insert(Item::new(
			Assignment {
				assignee,
				definition,
			},
			data,
		));
		self.model.items.push(index.into());
		(ItemRef::new(self.db, self.owner, index), source_map)
	}

	fn collect_constraint(&mut self, c: eprime::Constraint) {
		for expr in c.expressions() {
			let mut ctx = ExpressionCollector::new(self.db, self.identifiers, &mut self.diagnostics);
			let expression = ctx.collect_expression(expr);
			let (data, sm) = ctx.finish();
			let index = self.model.constraints.insert(Item::new(
				Constraint {
					annotations: Box::new([]),
					expression,
				},
				data,
			));
			self.model.items.push(index.into());
			let it = ItemRef::new(self.db, self.owner, index);
			self.source_map.insert(it.into(), Origin::new(&c));
			self.source_map.add_from_item_data(self.db, it, &sm);
		}
	}

	fn collect_objective(&mut self, o: eprime::Objective) -> (ItemRef, ItemDataSourceMap) {
		let mut ctx = ExpressionCollector::new(self.db, self.identifiers, &mut self.diagnostics);
		let goal = match o.strategy() {
			eprime::ObjectiveStrategy::Minimising => Goal::Minimize {
				pattern: ctx.alloc_pattern(
					Origin::new(&o.expression()),
					Pattern::Identifier(self.identifiers.objective),
				),
				objective: ctx.collect_expression(o.expression()),
			},
			eprime::ObjectiveStrategy::Maximising => Goal::Maximize {
				pattern: ctx.alloc_pattern(
					Origin::new(&o.expression()),
					Pattern::Identifier(self.identifiers.objective),
				),
				objective: ctx.collect_expression(o.expression()),
			},
		};
		let (data, source_map) = ctx.finish();
		let index = self
			.model
			.solves
			.insert(Item::new(Solve { goal, annotations: Box::new([])}, data));
		self.model.items.push(index.into());
		(ItemRef::new(self.db, self.owner, index), source_map)
	}


	// fn collect_domain_alias(&mut self, d: eprime::DomainAlias) -> (ItemRef, ItemDataSourceMap) {
	// 	let mut ctx = ExpressionCollector::new(self.db, self.identifiers, &mut self.diagnostics);
	// 	let name = ctx.collect_identifier_pattern(d.name());
	// 	let aliased_type = ctx.collect_domain(d.definition());
	// 	let (data, source_map) = ctx.finish();
	// 	let index = self.model.type_aliases.insert(Item::new(
	// 		TypeAlias {
	// 			name,
    // 			aliased_type,
    // 			annotations: Box::new([]),
	// 		},
	// 		data,
	// 	));
	// 	self.model.items.push(index.into());
	// 	(ItemRef::new(self.db, self.owner, index), source_map)
	// }

}