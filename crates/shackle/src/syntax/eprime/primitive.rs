//! AST representation of primitive values

use crate::syntax::ast::{
	ast_node, children_with_field_name, optional_child_with_field_name, AstNode, Children,
};

use super::{Domain, Expression};

ast_node!(
	/// Integer literal
	IntegerLiteral,
	value
);

impl IntegerLiteral {
	/// Get the value of this integer literal
	pub fn value(&self) -> i64 {
		self.cst_text().parse().unwrap()
	}
}

ast_node!(
	/// Boolean literal
	BooleanLiteral,
	value
);

impl BooleanLiteral {
	/// Get the value of this boolean literal
	pub fn value(&self) -> bool {
		match self.cst_text() {
			"true" => true,
			"false" => false,
			_ => unreachable!(),
		}
	}
}

ast_node!(
	/// Matrix Literal
	MatrixLiteral,
	members,
	index
);

impl MatrixLiteral {
	/// Get the members of this matrix literal
	pub fn members(&self) -> Children<'_, Expression> {
		children_with_field_name(self, "member")
	}

	/// Get the index of this matrix literal
	pub fn index(&self) -> Option<Domain> {
		optional_child_with_field_name(self, "index")
	}
}

#[cfg(test)]
mod test {
	use crate::syntax::ast::test::check_ast_eprime;
	use expect_test::expect;

	#[test]
	fn test_integer_literal() {
		check_ast_eprime(
			"letting one be 1",
			expect!([r#"
                EPrimeModel(
                    Model {
                        items: [
                            ConstDefinition(
                                ConstDefinition {
                                    cst_kind: "const_def",
                                    name: Identifier(
                                        Identifier {
                                            cst_kind: "identifier",
                                            name: "one",
                                        },
                                    ),
                                    definition: IntegerLiteral(
                                        IntegerLiteral {
                                            cst_kind: "integer_literal",
                                            value: 1,
                                        },
                                    ),
                                    domain: None,
                                },
                            ),
                        ],
                    },
                )
            "#]),
		);
	}

	#[test]
	fn test_boolean_literal() {
		check_ast_eprime(
			"letting T = true",
			expect!([r#"
            EPrimeModel(
                Model {
                    items: [
                        ConstDefinition(
                            ConstDefinition {
                                cst_kind: "const_def",
                                name: Identifier(
                                    Identifier {
                                        cst_kind: "identifier",
                                        name: "T",
                                    },
                                ),
                                definition: BooleanLiteral(
                                    BooleanLiteral {
                                        cst_kind: "boolean_literal",
                                        value: true,
                                    },
                                ),
                                domain: None,
                            },
                        ),
                    ],
                },
            )
            "#]),
		);
	}

	#[test]
	fn test_matrix_literal() {
		check_ast_eprime(
            "letting cmatrix: matrix indexed by [ int(1..2), int(1..4) ] of int(1..10) = [ [2,8,5,1], [3,7,9,4] ]",
            expect![[r#"
            EPrimeModel(
                Model {
                    items: [
                        ConstDefinition(
                            ConstDefinition {
                                cst_kind: "const_def",
                                name: Identifier(
                                    Identifier {
                                        cst_kind: "identifier",
                                        name: "cmatrix",
                                    },
                                ),
                                definition: MatrixLiteral(
                                    MatrixLiteral {
                                        cst_kind: "matrix_literal",
                                        members: [
                                            MatrixLiteral(
                                                MatrixLiteral {
                                                    cst_kind: "matrix_literal",
                                                    members: [
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 2,
                                                            },
                                                        ),
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 8,
                                                            },
                                                        ),
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 5,
                                                            },
                                                        ),
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 1,
                                                            },
                                                        ),
                                                    ],
                                                    index: None,
                                                },
                                            ),
                                            MatrixLiteral(
                                                MatrixLiteral {
                                                    cst_kind: "matrix_literal",
                                                    members: [
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 3,
                                                            },
                                                        ),
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 7,
                                                            },
                                                        ),
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 9,
                                                            },
                                                        ),
                                                        IntegerLiteral(
                                                            IntegerLiteral {
                                                                cst_kind: "integer_literal",
                                                                value: 4,
                                                            },
                                                        ),
                                                    ],
                                                    index: None,
                                                },
                                            ),
                                        ],
                                        index: None,
                                    },
                                ),
                                domain: Some(
                                    MatrixDomain(
                                        MatrixDomain {
                                            cst_kind: "matrix_domain",
                                            indexes: [
                                                IntegerDomain(
                                                    IntegerDomain {
                                                        cst_kind: "integer_domain",
                                                        range_members: [
                                                            InfixOperator(
                                                                InfixOperator {
                                                                    cst_kind: "infix_operator",
                                                                    operator: Operator {
                                                                        cst_kind: "..",
                                                                        name: "..",
                                                                    },
                                                                    left: IntegerLiteral(
                                                                        IntegerLiteral {
                                                                            cst_kind: "integer_literal",
                                                                            value: 1,
                                                                        },
                                                                    ),
                                                                    right: IntegerLiteral(
                                                                        IntegerLiteral {
                                                                            cst_kind: "integer_literal",
                                                                            value: 2,
                                                                        },
                                                                    ),
                                                                },
                                                            ),
                                                        ],
                                                    },
                                                ),
                                                IntegerDomain(
                                                    IntegerDomain {
                                                        cst_kind: "integer_domain",
                                                        range_members: [
                                                            InfixOperator(
                                                                InfixOperator {
                                                                    cst_kind: "infix_operator",
                                                                    operator: Operator {
                                                                        cst_kind: "..",
                                                                        name: "..",
                                                                    },
                                                                    left: IntegerLiteral(
                                                                        IntegerLiteral {
                                                                            cst_kind: "integer_literal",
                                                                            value: 1,
                                                                        },
                                                                    ),
                                                                    right: IntegerLiteral(
                                                                        IntegerLiteral {
                                                                            cst_kind: "integer_literal",
                                                                            value: 4,
                                                                        },
                                                                    ),
                                                                },
                                                            ),
                                                        ],
                                                    },
                                                ),
                                            ],
                                            base: IntegerDomain(
                                                IntegerDomain {
                                                    cst_kind: "integer_domain",
                                                    range_members: [
                                                        InfixOperator(
                                                            InfixOperator {
                                                                cst_kind: "infix_operator",
                                                                operator: Operator {
                                                                    cst_kind: "..",
                                                                    name: "..",
                                                                },
                                                                left: IntegerLiteral(
                                                                    IntegerLiteral {
                                                                        cst_kind: "integer_literal",
                                                                        value: 1,
                                                                    },
                                                                ),
                                                                right: IntegerLiteral(
                                                                    IntegerLiteral {
                                                                        cst_kind: "integer_literal",
                                                                        value: 10,
                                                                    },
                                                                ),
                                                            },
                                                        ),
                                                    ],
                                                },
                                            ),
                                        },
                                    ),
                                ),
                            },
                        ),
                    ],
                },
            )
            "#]]
        )
	}
}
