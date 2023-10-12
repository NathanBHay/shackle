use crate::constants::IdentifierRegistry;
use crate::file::ModelRef;
use crate::hir::db::Hir;
use crate::hir::lower::eprime ::ExpressionCollector;
use crate::hir::ids::ItemRef;
use crate::hir::source::{Origin, SourceMap};
use crate::hir::*;
use crate::syntax::eprime;
use crate::Error;

/// Collects AST items into an HIR model
pub struct ItemCollector<'a> {
	db: &'a dyn Hir,
	identifiers: &'a IdentifierRegistry,
	model: Model,
	source_map: SourceMap,
	diagnostics: Vec<Error>,
	owner: ModelRef,
	branching_annotations: Option<eprime::MatrixLiteral>, // Used to store branching annotations
	goal: eprime::Goal, // Used to store goal of solve
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
			branching_annotations: None,
			goal: eprime::Goal::Satisfy,
		}
	}

	/// Lower an AST item to HIR
	pub fn collect_item(&mut self, item: eprime::Item) {
        let (it, sm) = match item.clone() {
			eprime::Item::Constraint(c) => return self.collect_constraint(c),
			eprime::Item::ConstDefinition(c) => self.collect_const_definition(c),
			eprime::Item::DecisionDeclaration(d) => return self.collect_decision_declaration(d),
            eprime::Item::ParamDeclaration(p) => return self.collect_param_declaration(p),
			eprime::Item::DomainAlias(d) => self.collect_domain_alias(d),
			eprime::Item::Solve(o) => { self.goal = o.goal().clone(); return; },
			eprime::Item::Branching(b) => return self.collect_branching(b),
			eprime::Item::Heuristic(_) => return, // Currently not supported
			eprime::Item::Output(i) => self.collect_output(i)
		};
		self.source_map.insert(it.into(), Origin::new(&item));
		self.source_map.add_from_item_data(self.db, it, &sm);
	}

	/// Finish lowering
	pub fn finish(self) -> (Model, SourceMap, Vec<Error>) {
		(self.model, self.source_map, self.diagnostics)
	}

	/// Checks if a solve item exists, if not, adds satisfy solve
	/// TODO: Broken SourceMap
	pub fn add_solve(&mut self) {
		let mut ctx= ExpressionCollector::new(self.db, &mut self.diagnostics);

		let annotations = match &self.branching_annotations {
			Some(b) => {
				let origin = Origin::new(b);
				let arguments = Box::new([
					ctx.collect_matrix_literal(b.clone()),
					ctx.alloc_expression(origin.clone(), Identifier::new("input_order", self.db)),
					ctx.alloc_expression(origin.clone(), Identifier::new("indomain_min", self.db))
				]);
				let search = ctx.alloc_expression(origin.clone(), Identifier::new("int_search", self.db));
				Box::new([ctx.alloc_expression(origin.clone(), Call { function: search, arguments })])
			},
			None => Box::new([]) as Box<[ArenaIndex<Expression>]>
		};
		let goal = match &self.goal {
			eprime::Goal::Satisfy => Goal::Satisfy,
			eprime::Goal::Minimising(e) => Goal::Minimize {
				pattern: ctx.alloc_pattern(
					Origin::new(e),
					Pattern::Identifier(self.identifiers.objective),
				),
				objective: ctx.collect_expression(e.clone())
			},
			eprime::Goal::Maximising(e) => Goal::Maximize {
				pattern: ctx.alloc_pattern(
					Origin::new(e),
					Pattern::Identifier(self.identifiers.objective),
				),
				objective: ctx.collect_expression(e.clone())
			},
		};
		let (data, _) = ctx.finish();
		let index = self
			.model
			.solves
			.insert(Item::new(Solve { goal, annotations }, data));
		self.model.items.insert(self.model.items.len().checked_sub(1).unwrap_or(0), index.into());
		// let it = ItemRef::new(self.db, self.owner, index);
		// self.source_map.insert(it.into(), Origin::new(&goal));
		// self.source_map.add_from_item_data(self.db, it, &sm);
	}

	fn collect_const_definition(&mut self, c: eprime::ConstDefinition) -> (ItemRef, ItemDataSourceMap) {
		let mut ctx = ExpressionCollector::new(self.db, &mut self.diagnostics);
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

	fn collect_param_declaration(&mut self, p: eprime::ParamDeclaration) {
		self.collect_declarations(p.names(), p.domain());

		// Collect where expression as constraint
		if p.wheres().is_some() {
			self.collect_constraint_expression(p.wheres().unwrap());
		}
	}

	fn collect_decision_declaration(&mut self, d: eprime::DecisionDeclaration) {
		self.collect_declarations(d.names(), d.domain());
	}

	fn collect_declarations<I: Iterator<Item = eprime::Identifier>>(&mut self, names: I, domain: eprime::Domain) {
		for name in names {
			let mut ctx = ExpressionCollector::new(self.db, &mut self.diagnostics);
			let declared_type = ctx.collect_domain(domain.clone());
			let pattern = ctx.alloc_ident_pattern(Origin::new(&name), name.clone());
			let (data, sm) = ctx.finish();
			let index = self.model.declarations.insert(Item::new(
				Declaration {
					declared_type,
					pattern,
					definition: None,
					annotations: Box::new([]),
				},
				data,
			));
			self.model.items.push(index.into());
			let it = ItemRef::new(self.db, self.owner, index);
			self.source_map.insert(it.into(), Origin::new(&name));
			self.source_map.add_from_item_data(self.db, it, &sm);
		}
	}

	fn collect_constraint(&mut self, c: eprime::Constraint) {
		for expr in c.expressions() {
			self.collect_constraint_expression(expr);
		}
	}

	fn collect_constraint_expression(&mut self, expr: eprime::Expression) {
		let mut ctx = ExpressionCollector::new(self.db,  &mut self.diagnostics);
		let expression = ctx.collect_expression(expr.clone());
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
		self.source_map.insert(it.into(), Origin::new(&expr));
		self.source_map.add_from_item_data(self.db, it, &sm);
	}

	fn collect_branching(&mut self, b: eprime::Branching) {
		self.branching_annotations = Some(b.branching_array());
	}

	fn collect_domain_alias(&mut self, d: eprime::DomainAlias) -> (ItemRef, ItemDataSourceMap) {
		let origin = Origin::new(&d);
		let mut ctx = ExpressionCollector::new(self.db, &mut self.diagnostics);
		let name = ctx.alloc_ident_pattern(origin.clone(), d.name());
		let aliased_type = ctx.collect_domain(d.definition());
		let (data, source_map) = ctx.finish();
		let index = self.model.type_aliases.insert(Item::new(
			TypeAlias {
				name,
    			aliased_type,
    			annotations: Box::new([]),
			},
			data,
		));
		self.model.items.push(index.into());
		(ItemRef::new(self.db, self.owner, index), source_map)
	}

	fn collect_output(&mut self, i: eprime::Output) -> (ItemRef, ItemDataSourceMap) {
		let mut ctx = ExpressionCollector::new(self.db, &mut self.diagnostics);
		let expression = ctx.collect_expression(i.expression());
		let (data, source_map) = ctx.finish();
		let index = self.model.outputs.insert(Item::new(
			Output {
				section: None,
				expression,
			},
			data,
		));
		self.model.items.push(index.into());
		(ItemRef::new(self.db, self.owner, index), source_map)
	}	
}