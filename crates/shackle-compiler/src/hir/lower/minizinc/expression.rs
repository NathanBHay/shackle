use rustc_hash::FxHashMap;

use crate::{
	constants::IdentifierRegistry,
	db::InternedStringData,
	diagnostics::{InvalidArrayLiteral, InvalidNumericLiteral, SyntaxError},
	hir::{db::Hir, source::Origin, *},
	syntax::{ast::AstNode, minizinc},
	utils::{arena::ArenaIndex, maybe_grow_stack},
	Error,
};

/// Collects AST expressions for owned by an item and lowers them into HIR recursively.
pub struct ExpressionCollector<'a> {
	db: &'a dyn Hir,
	identifiers: &'a IdentifierRegistry,
	data: ItemData,
	source_map: ItemDataSourceMap,
	diagnostics: &'a mut Vec<Error>,
}

impl ExpressionCollector<'_> {
	/// Create a new expression collector
	pub fn new<'a>(
		db: &'a dyn Hir,
		identifiers: &'a IdentifierRegistry,
		diagnostics: &'a mut Vec<Error>,
	) -> ExpressionCollector<'a> {
		ExpressionCollector {
			db,
			identifiers,
			data: ItemData::new(),
			source_map: ItemDataSourceMap::new(),
			diagnostics,
		}
	}

	/// Add a diagnostic
	pub fn add_diagnostic<E: Into<Error>>(&mut self, error: E) {
		self.diagnostics.push(error.into());
	}

	/// Lower an AST expression into HIR
	pub fn collect_expression(
		&mut self,
		expression: minizinc::Expression,
	) -> ArenaIndex<Expression> {
		maybe_grow_stack(|| self.collect_expression_inner(expression))
	}

	fn collect_expression_inner(
		&mut self,
		expression: minizinc::Expression,
	) -> ArenaIndex<Expression> {
		let origin = Origin::new(&expression);
		log::debug!(
			"Lowering {} to HIR ({})",
			expression.cst_node().text(),
			origin.debug_print(self.db)
		);
		if expression.is_missing() {
			return self.alloc_expression(origin, Expression::Missing);
		}
		let collected: Expression = match expression {
			minizinc::Expression::IntegerLiteral(i) => {
				IntegerLiteral(i.value().unwrap_or_else(|e| {
					let (src, span) = i.cst_node().source_span(self.db.upcast());
					self.add_diagnostic(InvalidNumericLiteral {
						src,
						span,
						msg: e.to_string(),
					});
					0
				}))
				.into()
			}
			minizinc::Expression::FloatLiteral(f) => {
				FloatLiteral::new(f.value().unwrap_or_else(|e| {
					let (src, span) = f.cst_node().source_span(self.db.upcast());
					self.add_diagnostic(InvalidNumericLiteral {
						src,
						span,
						msg: e.to_string(),
					});
					0.0
				}))
				.into()
			}
			minizinc::Expression::BooleanLiteral(b) => BooleanLiteral(b.value()).into(),
			minizinc::Expression::StringLiteral(s) => StringLiteral::new(s.value(), self.db).into(),
			minizinc::Expression::Absent(_) => Expression::Absent,
			minizinc::Expression::Infinity(_) => Expression::Infinity,
			minizinc::Expression::Anonymous(a) => {
				// No longer support anonymous variables, instead use opt
				let (src, span) = a.cst_node().source_span(self.db.upcast());
				self.add_diagnostic(SyntaxError {
					src,
					span,
					msg: "Anonymous variables in expressions are not supported".to_string(),
					other: Vec::new(),
				});
				Expression::Missing
			}
			minizinc::Expression::Identifier(i) => Identifier::new(i.name(), self.db).into(),
			minizinc::Expression::TupleLiteral(t) => self.collect_tuple_literal(t).into(),
			minizinc::Expression::RecordLiteral(r) => self.collect_record_literal(r).into(),
			minizinc::Expression::SetLiteral(sl) => self.collect_set_literal(sl).into(),
			minizinc::Expression::ArrayLiteral(al) => return self.collect_array_literal(al),
			minizinc::Expression::ArrayLiteral2D(al) => return self.collect_2d_array_literal(al),
			minizinc::Expression::ArrayAccess(aa) => self.collect_array_access(aa).into(),
			minizinc::Expression::ArrayComprehension(c) => {
				self.collect_array_comprehension(c).into()
			}
			minizinc::Expression::SetComprehension(c) => self.collect_set_comprehension(c).into(),
			minizinc::Expression::IfThenElse(i) => self.collect_if_then_else(i).into(),
			minizinc::Expression::Call(c) => self.collect_call(c).into(),
			minizinc::Expression::InfixOperator(o) => return self.collect_infix_operator(o),
			minizinc::Expression::PrefixOperator(o) => return self.collect_prefix_operator(o),
			minizinc::Expression::PostfixOperator(o) => return self.collect_postfix_operator(o),
			minizinc::Expression::GeneratorCall(c) => return self.collect_generator_call(c),
			minizinc::Expression::StringInterpolation(s) => {
				return self.collect_string_interpolation(s)
			}
			minizinc::Expression::Case(c) => self.collect_case(c).into(),
			minizinc::Expression::Let(l) => self.collect_let(l).into(),
			minizinc::Expression::TupleAccess(t) => self.collect_tuple_access(t).into(),
			minizinc::Expression::RecordAccess(t) => self.collect_record_access(t).into(),
			minizinc::Expression::Lambda(l) => self.collect_lambda(l).into(),
			minizinc::Expression::AnnotatedExpression(e) => {
				return self.collect_annotated_expression(e)
			}
		};
		self.alloc_expression(origin, collected)
	}

	/// Lower an AST type into HIR
	pub fn collect_type(&mut self, t: minizinc::Type) -> ArenaIndex<Type> {
		let mut tiids = TypeInstIdentifiers::default();
		self.collect_type_with_tiids(t, &mut tiids, false, false)
	}

	/// Lower an AST type into HIR and collect implicit type inst ID declarations
	pub fn collect_type_with_tiids(
		&mut self,
		t: minizinc::Type,
		tiids: &mut TypeInstIdentifiers,
		is_array_dim: bool,
		is_fn_parameter: bool,
	) -> ArenaIndex<Type> {
		let origin = Origin::new(&t);
		if t.is_missing() {
			return self.alloc_type(origin, Type::Missing);
		}
		let ty = match t {
			minizinc::Type::ArrayType(a) => Type::Array {
				opt: OptType::NonOpt,
				dimensions: {
					let dims: Box<[_]> = a
						.dimensions()
						.map(|dim| self.collect_type_with_tiids(dim, tiids, true, is_fn_parameter))
						.collect();
					if dims.len() == 1 {
						dims[0]
					} else {
						self.alloc_type(
							origin.clone(),
							Type::Tuple {
								opt: OptType::NonOpt,
								fields: dims,
							},
						)
					}
				},
				element: self.collect_type_with_tiids(
					a.element_type(),
					tiids,
					false,
					is_fn_parameter,
				),
			},
			minizinc::Type::SetType(s) => Type::Set {
				inst: s.var_type(),
				opt: s.opt_type(),
				element: self.collect_type_with_tiids(
					s.element_type(),
					tiids,
					false,
					is_fn_parameter,
				),
			},
			minizinc::Type::TupleType(t) => Type::Tuple {
				opt: OptType::NonOpt,
				fields: t
					.fields()
					.map(|f| self.collect_type_with_tiids(f, tiids, false, is_fn_parameter))
					.collect(),
			},
			minizinc::Type::RecordType(r) => Type::Record {
				opt: OptType::NonOpt,
				fields: r
					.fields()
					.map(|f| {
						(
							self.collect_pattern(f.name().into()),
							self.collect_type_with_tiids(
								f.field_type(),
								tiids,
								false,
								is_fn_parameter,
							),
						)
					})
					.collect(),
			},
			minizinc::Type::OperationType(o) => Type::Operation {
				opt: OptType::NonOpt,
				return_type: self.collect_type_with_tiids(
					o.return_type(),
					tiids,
					false,
					is_fn_parameter,
				),
				parameter_types: o
					.parameter_types()
					.map(|p| self.collect_type_with_tiids(p, tiids, false, is_fn_parameter))
					.collect(),
			},
			minizinc::Type::TypeBase(b) => {
				self.collect_type_base(b, tiids, is_array_dim, is_fn_parameter)
			}
			minizinc::Type::AnyType(_) => Type::Any,
		};
		self.alloc_type(origin, ty)
	}

	/// Lower an AST pattern into HIR
	pub fn collect_pattern(&mut self, p: minizinc::Pattern) -> ArenaIndex<Pattern> {
		let origin = Origin::new(&p);
		if p.is_missing() {
			return self.alloc_pattern(origin, Pattern::Missing);
		}
		match p {
			minizinc::Pattern::Identifier(i) => {
				let identifier = Identifier::new(i.name(), self.db);
				self.alloc_pattern(origin, identifier)
			}
			minizinc::Pattern::Anonymous(_) => self.alloc_pattern(origin, Pattern::Anonymous),
			minizinc::Pattern::Absent(_) => self.alloc_pattern(origin, Pattern::Absent),
			minizinc::Pattern::BooleanLiteral(b) => {
				self.alloc_pattern(origin, Pattern::Boolean(BooleanLiteral(b.value())))
			}
			minizinc::Pattern::StringLiteral(s) => self.alloc_pattern(
				origin,
				Pattern::String(StringLiteral::new(s.value(), self.db)),
			),
			minizinc::Pattern::PatternNumericLiteral(n) => match n.value() {
				minizinc::NumericLiteral::IntegerLiteral(i) => {
					let pat = Pattern::Integer {
						negated: n.negated(),
						value: IntegerLiteral(i.value().unwrap_or_else(|e| {
							let (src, span) = i.cst_node().source_span(self.db.upcast());
							self.add_diagnostic(InvalidNumericLiteral {
								src,
								span,
								msg: e.to_string(),
							});
							0
						})),
					};
					self.alloc_pattern(origin, pat)
				}
				minizinc::NumericLiteral::FloatLiteral(f) => {
					let pat = Pattern::Float {
						negated: n.negated(),
						value: FloatLiteral::new(f.value().unwrap_or_else(|e| {
							let (src, span) = f.cst_node().source_span(self.db.upcast());
							self.add_diagnostic(InvalidNumericLiteral {
								src,
								span,
								msg: e.to_string(),
							});
							0.0
						})),
					};
					self.alloc_pattern(origin, pat)
				}
				minizinc::NumericLiteral::Infinity(_) => self.alloc_pattern(
					origin,
					Pattern::Infinity {
						negated: n.negated(),
					},
				),
			},
			minizinc::Pattern::Call(c) => {
				let ident = c.identifier();
				let pattern = Pattern::Call {
					function: self
						.alloc_pattern(Origin::new(&ident), Identifier::new(ident.name(), self.db)),
					arguments: c.arguments().map(|a| self.collect_pattern(a)).collect(),
				};
				self.alloc_pattern(origin, pattern)
			}
			minizinc::Pattern::Tuple(t) => {
				let pattern = Pattern::Tuple {
					fields: t.fields().map(|f| self.collect_pattern(f)).collect(),
				};
				self.alloc_pattern(origin, pattern)
			}
			minizinc::Pattern::Record(r) => {
				let pattern = Pattern::Record {
					fields: r
						.fields()
						.map(|f| {
							(
								Identifier::new(f.name().name(), self.db),
								self.collect_pattern(f.value()),
							)
						})
						.collect(),
				};
				self.alloc_pattern(origin, pattern)
			}
		}
	}

	/// Get the collected expressions
	pub fn finish(mut self) -> (ItemData, ItemDataSourceMap) {
		self.data.shrink_to_fit();
		(self.data, self.source_map)
	}

	fn collect_type_base(
		&mut self,
		b: minizinc::TypeBase,
		tiids: &mut TypeInstIdentifiers,
		is_array_dim: bool,
		is_fn_parameter: bool,
	) -> Type {
		match b.domain() {
			minizinc::Domain::Bounded(e) => {
				if is_array_dim && b.var_type().is_none() && b.opt_type().is_none() {
					if let minizinc::Expression::Anonymous(_) = e {
						if is_fn_parameter {
							let pattern =
								self.alloc_pattern(Origin::new(&e), Identifier::new("_", self.db));
							tiids.anons.push(TypeInstIdentifierDeclaration {
								name: pattern,
								anonymous: true,
								is_enum: true,
								is_varifiable: true,
								is_indexable: false,
							});
							return Type::AnonymousTypeInstVar {
								inst: Some(VarType::Par),
								opt: Some(OptType::NonOpt),
								pattern,
							};
						} else {
							return Type::Any;
						}
					}
				}
				Type::Bounded {
					inst: b.var_type(),
					opt: b.opt_type(),
					domain: self.collect_expression(e),
				}
			}
			minizinc::Domain::Unbounded(u) => Type::Primitive {
				inst: b.var_type().unwrap_or(VarType::Par),
				opt: b.opt_type().unwrap_or(OptType::NonOpt),
				primitive_type: u.primitive_type(),
			},
			minizinc::Domain::TypeInstIdentifier(tiid) => {
				let ident = Identifier::new(tiid.name(), self.db);
				let origin = Origin::new(&tiid);
				let (inst, opt) = match (b.any_type(), b.var_type(), b.opt_type()) {
					(true, _, _) => (None, None), // Unrestricted
					(_, None, None) => (Some(VarType::Par), Some(OptType::NonOpt)), // No prefix means par non-opt
					(_, None, o) => (Some(VarType::Par), o), // opt prefix means par opt
					(_, i, None) => (i, Some(OptType::NonOpt)), // var prefix means var non-opt
					(_, i, o) => (i, o),          // var opt means var opt
				};
				tiids
					.tiids
					.entry(ident)
					.and_modify(|(in_param, tiid)| {
						tiid.is_varifiable =
							tiid.is_varifiable || inst == Some(VarType::Var) || is_array_dim;
						tiid.is_indexable = tiid.is_indexable || is_array_dim;
						*in_param = *in_param || is_fn_parameter;
					})
					.or_insert((
						is_fn_parameter,
						TypeInstIdentifierDeclaration {
							name: self.alloc_pattern(origin.clone(), ident),
							anonymous: false,
							is_enum: false,
							is_varifiable: inst == Some(VarType::Var) || is_array_dim,
							is_indexable: is_array_dim,
						},
					));
				Type::Bounded {
					inst,
					opt,
					domain: self.alloc_expression(origin, ident),
				}
			}
			minizinc::Domain::TypeInstEnumIdentifier(tiid) => {
				let ident = Identifier::new(tiid.name(), self.db);
				let origin = Origin::new(&tiid);
				tiids
					.tiids
					.entry(ident)
					.and_modify(|(in_param, tiid)| {
						tiid.is_enum = true;
						*in_param = *in_param || is_fn_parameter;
					})
					.or_insert((
						is_fn_parameter,
						TypeInstIdentifierDeclaration {
							name: self.alloc_pattern(origin.clone(), ident),
							anonymous: false,
							is_enum: true,
							is_varifiable: true,
							is_indexable: false,
						},
					));
				let (inst, opt) = match (b.any_type(), b.var_type(), b.opt_type()) {
					(true, _, _) => (None, None), // Unrestricted
					(_, None, None) => (Some(VarType::Par), Some(OptType::NonOpt)), // No prefix means par non-opt
					(_, None, o) => (Some(VarType::Par), o), // opt prefix means par opt
					(_, i, None) => (i, Some(OptType::NonOpt)), // var prefix means var non-opt
					(_, i, o) => (i, o),          // var opt means var opt
				};
				Type::Bounded {
					inst,
					opt,
					domain: self.alloc_expression(origin, ident),
				}
			}
		}
	}

	fn collect_set_literal(&mut self, sl: minizinc::SetLiteral) -> SetLiteral {
		SetLiteral {
			members: sl.members().map(|e| self.collect_expression(e)).collect(),
		}
	}

	fn collect_array_literal(&mut self, al: minizinc::ArrayLiteral) -> ArenaIndex<Expression> {
		let (indices, values): (Vec<_>, Vec<_>) = al
			.members()
			.map(|m| {
				(
					m.indices().map(|i| self.collect_expression(i)),
					self.collect_expression(m.value()),
				)
			})
			.unzip();
		if indices.iter().all(|is| is.is_none()) {
			// Non-indexed
			self.alloc_expression(
				Origin::new(&al),
				ArrayLiteral {
					members: values.into_boxed_slice(),
				},
			)
		} else {
			let mut start_indexed = indices[0].is_some();
			let mut fully_indexed = start_indexed;
			for is in indices[1..].iter() {
				if is.is_some() {
					start_indexed = false;
				} else {
					fully_indexed = false;
				}
				if !start_indexed && !fully_indexed {
					let (src, span) = al.cst_node().source_span(self.db.upcast());
					self.add_diagnostic(InvalidArrayLiteral {
						src,
						span,
						msg: "Indexed array literal must be fully indexed, or only have an index for the first element".to_string(),
					});
					return self.alloc_expression(Origin::new(&al), Expression::Missing);
				}
			}
			self.alloc_expression(
				Origin::new(&al),
				IndexedArrayLiteral {
					indices: indices.into_iter().flatten().collect(),
					members: values.into_boxed_slice(),
				},
			)
		}
	}

	fn collect_2d_array_literal(&mut self, al: minizinc::ArrayLiteral2D) -> ArenaIndex<Expression> {
		// Desugar into array2d call
		let origin = Origin::new(&al);
		let col_indices = al
			.column_indices()
			.map(|i| self.collect_expression(i))
			.collect::<Vec<_>>();
		let mut first = true;
		let mut col_count = 0;
		let mut row_indices = Vec::new();
		let mut row_count = 0;
		let mut values = Vec::new();
		for row in al.rows() {
			let members = row
				.members()
				.map(|m| self.collect_expression(m))
				.collect::<Vec<_>>();
			let index = row.index();
			if let Some(ref i) = index {
				row_indices.push(self.collect_expression(i.clone()));
			}

			if first {
				col_count = members.len();
				first = false;

				if !col_indices.is_empty() && col_count != col_indices.len() {
					let (src, span) = al.cst_node().source_span(self.db.upcast());
					self.add_diagnostic(InvalidArrayLiteral {
						src,
						span,
						msg: "2D array literal has different row length to index row".to_string(),
					});
					return self.alloc_expression(origin, Expression::Missing);
				}
			} else if members.len() != col_count {
				let (src, span) = al.cst_node().source_span(self.db.upcast());
				self.add_diagnostic(InvalidArrayLiteral {
					src,
					span,
					msg: "Non-uniform 2D array literal row length".to_string(),
				});
				return self.alloc_expression(origin, Expression::Missing);
			}

			if index.is_none() != row_indices.is_empty() {
				let (src, span) = al.cst_node().source_span(self.db.upcast());
				self.add_diagnostic(InvalidArrayLiteral {
					src,
					span,
					msg: "Mixing indexed and non-indexed rows not allowed".to_string(),
				});
				return self.alloc_expression(origin, Expression::Missing);
			}

			values.extend(members);
			row_count += 1;
		}

		self.alloc_expression(
			origin,
			ArrayLiteral2D {
				rows: if row_indices.is_empty() {
					MaybeIndexSet::NonIndexed(row_count)
				} else {
					MaybeIndexSet::Indexed(row_indices.into_boxed_slice())
				},
				columns: if col_indices.is_empty() {
					MaybeIndexSet::NonIndexed(col_count)
				} else {
					MaybeIndexSet::Indexed(col_indices.into_boxed_slice())
				},
				members: values.into_boxed_slice(),
			},
		)
	}

	fn collect_array_access(&mut self, aa: minizinc::ArrayAccess) -> ArrayAccess {
		let indices = aa
			.indices()
			.map(|i| match i {
				minizinc::ArrayIndex::Expression(e) => self.collect_expression(e),
				minizinc::ArrayIndex::IndexSlice(s) => self.alloc_expression(
					Origin::new(&s),
					Expression::Slice(Identifier::new(s.operator(), self.db)),
				),
			})
			.collect::<Box<[_]>>();
		ArrayAccess {
			collection: self.collect_expression(aa.collection()),
			indices: if indices.len() == 1 {
				indices[0]
			} else {
				self.alloc_expression(Origin::new(&aa), TupleLiteral { fields: indices })
			},
		}
	}

	fn collect_array_comprehension(
		&mut self,
		c: minizinc::ArrayComprehension,
	) -> ArrayComprehension {
		ArrayComprehension {
			generators: c.generators().map(|g| self.collect_generator(g)).collect(),
			indices: c.indices().map(|i| self.collect_expression(i)),
			template: self.collect_expression(c.template()),
		}
	}

	fn collect_set_comprehension(&mut self, c: minizinc::SetComprehension) -> SetComprehension {
		SetComprehension {
			generators: c.generators().map(|g| self.collect_generator(g)).collect(),
			template: self.collect_expression(c.template()),
		}
	}

	fn collect_generator(&mut self, g: minizinc::Generator) -> Generator {
		match g {
			minizinc::Generator::IteratorGenerator(i) => Generator::Iterator {
				patterns: i.patterns().map(|p| self.collect_pattern(p)).collect(),
				collection: self.collect_expression(i.collection()),
				where_clause: i.where_clause().map(|w| self.collect_expression(w)),
			},
			minizinc::Generator::AssignmentGenerator(a) => Generator::Assignment {
				pattern: self.collect_pattern(a.pattern()),
				value: self.collect_expression(a.value()),
				where_clause: a.where_clause().map(|w| self.collect_expression(w)),
			},
		}
	}

	fn collect_if_then_else(&mut self, ite: minizinc::IfThenElse) -> IfThenElse {
		IfThenElse {
			branches: ite
				.branches()
				.map(|b| Branch {
					condition: self.collect_expression(b.condition),
					result: self.collect_expression(b.result),
				})
				.collect(),
			else_result: ite.else_result().map(|e| self.collect_expression(e)),
		}
	}

	fn collect_call(&mut self, c: minizinc::Call) -> Call {
		Call {
			arguments: c.arguments().map(|a| self.collect_expression(a)).collect(),
			function: self.collect_expression(c.function()),
		}
	}

	fn collect_infix_operator(&mut self, o: minizinc::InfixOperator) -> ArenaIndex<Expression> {
		let arguments = [o.left(), o.right()]
			.into_iter()
			.map(|a| self.collect_expression(a))
			.collect();
		let operator = o.operator();
		let function = self.ident_exp(
			Origin::new(&operator),
			if operator.name() == "==" {
				// Desugar == into =
				"="
			} else {
				operator.name()
			},
		);
		self.alloc_expression(
			Origin::new(&o),
			Call {
				function,
				arguments,
			},
		)
	}

	fn collect_prefix_operator(&mut self, o: minizinc::PrefixOperator) -> ArenaIndex<Expression> {
		let arguments = Box::new([self.collect_expression(o.operand())]);
		let operator = o.operator();
		let function = self.ident_exp(Origin::new(&operator), operator.name());
		self.alloc_expression(
			Origin::new(&o),
			Call {
				function,
				arguments,
			},
		)
	}

	fn collect_postfix_operator(&mut self, o: minizinc::PostfixOperator) -> ArenaIndex<Expression> {
		let arguments = Box::new([self.collect_expression(o.operand())]);
		let operator = o.operator();
		let function = self.ident_exp(Origin::new(&operator), format!("{}o", operator.name()));
		self.alloc_expression(
			Origin::new(&o),
			Call {
				function,
				arguments,
			},
		)
	}

	fn collect_generator_call(&mut self, c: minizinc::GeneratorCall) -> ArenaIndex<Expression> {
		// Desugar into call with comprehension as argument
		let origin = Origin::new(&c);
		let comp = ArrayComprehension {
			generators: c.generators().map(|g| self.collect_generator(g)).collect(),
			indices: None,
			template: self.collect_expression(c.template()),
		};
		let arguments = Box::new([self.alloc_expression(origin.clone(), comp)]);
		let function = self.collect_expression(c.function());
		self.alloc_expression(
			origin,
			Call {
				arguments,
				function,
			},
		)
	}

	fn collect_string_interpolation(
		&mut self,
		s: minizinc::StringInterpolation,
	) -> ArenaIndex<Expression> {
		// Desugar into concat() of show() calls
		let origin = Origin::new(&s);
		let strings = s
			.contents()
			.map(|c| match c {
				minizinc::InterpolationItem::String(v) => {
					self.alloc_expression(origin.clone(), StringLiteral::new(v, self.db))
				}
				minizinc::InterpolationItem::Expression(e) => {
					let arguments = Box::new([self.collect_expression(e.clone())]);
					let function = self.alloc_expression(Origin::new(&e), self.identifiers.show);
					self.alloc_expression(
						Origin::new(&e),
						Call {
							function,
							arguments,
						},
					)
				}
			})
			.collect();
		let arguments =
			Box::new([self.alloc_expression(origin.clone(), ArrayLiteral { members: strings })]);
		let function = self.alloc_expression(origin.clone(), self.identifiers.concat);

		self.alloc_expression(
			origin,
			Call {
				function,
				arguments,
			},
		)
	}

	fn collect_case(&mut self, c: minizinc::Case) -> Case {
		let expression = self.collect_expression(c.expression());
		let cases = c
			.cases()
			.map(|i| CaseItem {
				pattern: self.collect_pattern(i.pattern()),
				value: self.collect_expression(i.value()),
			})
			.collect();
		Case { expression, cases }
	}

	fn collect_let(&mut self, l: minizinc::Let) -> Let {
		let items = l.items().map(|i| self.collect_let_item(i)).collect();
		let in_expression = self.collect_expression(l.in_expression());
		Let {
			items,
			in_expression,
		}
	}

	fn collect_let_item(&mut self, i: minizinc::LetItem) -> LetItem {
		match i {
			minizinc::LetItem::Declaration(d) => {
				let declared_type = self.collect_type(d.declared_type());
				Declaration {
					pattern: self.collect_pattern(d.pattern()),
					definition: d.definition().map(|def| self.collect_expression(def)),
					declared_type,
					annotations: d
						.annotations()
						.map(|ann| self.collect_expression(ann))
						.collect(),
				}
				.into()
			}
			minizinc::LetItem::Constraint(c) => Constraint {
				expression: self.collect_expression(c.expression()),
				annotations: c
					.annotations()
					.map(|ann| self.collect_expression(ann))
					.collect(),
			}
			.into(),
		}
	}

	fn collect_tuple_literal(&mut self, t: minizinc::TupleLiteral) -> TupleLiteral {
		TupleLiteral {
			fields: t.members().map(|m| self.collect_expression(m)).collect(),
		}
	}

	fn collect_record_literal(&mut self, r: minizinc::RecordLiteral) -> RecordLiteral {
		RecordLiteral {
			fields: r
				.members()
				.map(|m| {
					(
						self.collect_pattern(m.name().into()),
						self.collect_expression(m.value()),
					)
				})
				.collect(),
		}
	}

	fn collect_tuple_access(&mut self, t: minizinc::TupleAccess) -> TupleAccess {
		TupleAccess {
			field: IntegerLiteral(t.field().value().unwrap_or_else(|e| {
				let (src, span) = t.field().cst_node().source_span(self.db.upcast());
				self.add_diagnostic(InvalidNumericLiteral {
					src,
					span,
					msg: e.to_string(),
				});
				1
			})),
			tuple: self.collect_expression(t.tuple()),
		}
	}

	fn collect_record_access(&mut self, r: minizinc::RecordAccess) -> RecordAccess {
		RecordAccess {
			record: self.collect_expression(r.record()),
			field: Identifier::new(r.field().name(), self.db),
		}
	}

	fn collect_lambda(&mut self, l: minizinc::Lambda) -> Lambda {
		Lambda {
			return_type: l.return_type().map(|r| self.collect_type(r)),
			parameters: l
				.parameters()
				.map(|p| {
					let ty = self.collect_type(p.declared_type());
					let annotations = p
						.annotations()
						.map(|ann| self.collect_expression(ann))
						.collect();
					let pattern = p.pattern().map(|p| self.collect_pattern(p));
					Parameter {
						declared_type: ty,
						pattern,
						annotations,
					}
				})
				.collect(),
			body: self.collect_expression(l.body()),
		}
	}

	fn collect_annotated_expression(
		&mut self,
		e: minizinc::AnnotatedExpression,
	) -> ArenaIndex<Expression> {
		let annotations = e
			.annotations()
			.map(|ann| self.collect_expression(ann))
			.collect();
		let idx = self.collect_expression(e.expression());
		self.data.annotations.insert(idx, annotations);
		idx
	}

	fn ident_exp<T: Into<InternedStringData>>(
		&mut self,
		origin: Origin,
		id: T,
	) -> ArenaIndex<Expression> {
		self.alloc_expression(origin, Identifier::new(id, self.db))
	}

	pub(super) fn alloc_expression<V: Into<Expression>>(
		&mut self,
		origin: Origin,
		v: V,
	) -> ArenaIndex<Expression> {
		let index = self.data.expressions.insert(v.into());
		self.source_map.expression_source.insert(index, origin);
		index
	}

	pub(super) fn alloc_type<V: Into<Type>>(&mut self, origin: Origin, v: V) -> ArenaIndex<Type> {
		let index = self.data.types.insert(v);
		self.source_map.type_source.insert(index, origin);
		index
	}

	pub(super) fn alloc_pattern<V: Into<Pattern>>(
		&mut self,
		origin: Origin,
		v: V,
	) -> ArenaIndex<Pattern> {
		let index = self.data.patterns.insert(v);
		self.source_map.pattern_source.insert(index, origin);
		index
	}
}

/// Tracks type-inst identifiers used in a function item
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TypeInstIdentifiers {
	/// The named type-inst ids
	pub tiids: FxHashMap<Identifier, (bool, TypeInstIdentifierDeclaration)>,
	/// Anonymous type-inst ids
	pub anons: Vec<TypeInstIdentifierDeclaration>,
}

impl TypeInstIdentifiers {
	/// Get the `TypeInstIdentifierDeclaration`s
	pub fn into_vec(self) -> Vec<TypeInstIdentifierDeclaration> {
		let mut tiids = self
			.tiids
			.into_values()
			.filter_map(|(ok, d)| if ok { Some(d) } else { None })
			.chain(self.anons)
			.collect::<Vec<_>>();
		tiids.sort_by_key(|tiid| tiid.name);
		tiids
	}
}
