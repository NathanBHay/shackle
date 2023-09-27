//! AST representation of Eprime Expressions

use super::{
    IntegerLiteral, BooleanLiteral,
};

use crate::syntax::ast::{
    ast_enum, ast_node, AstNode,
}; 

ast_enum!(
    /// Expression
    Expression,
    "boolean_literal" => BooleanLiteral,
    // "call" => Call,
    "identifier" => Identifier,
    // "indexed_access" => IndexedAccess,
    // "infix_operator" => InfixOperator,
    // "set_in" => SetIn,
    "integer_literal" => IntegerLiteral,
    // "matrix_literal" => MatrixLiteral,
    // "prefix_operator" => PrefixOperator,
    // "quantification" => Quantification,
    // "matrix_comprehension" => MatrixComprehension,
    // "absolute_operator" => AbsoluteOperator,
);

ast_node!(
    // Identifier
    Identifier,
    name
);

impl Identifier {
    /// Get the name of this identifier
    pub fn name(&self) -> &str {
        self.cst_text()
    }
}

#[cfg(test)]
mod test {
    use crate::syntax::ast::test::*;
    use expect_test::expect;

    #[test]
    fn test_identifier() {
        check_ast(
            "letting x = a",
            expect![[r#"
    Model {
        items: [
            Assignment(
                Assignment {
                    cst_kind: "assignment",
                    assignee: Identifier(
                        UnquotedIdentifier(
                            UnquotedIdentifier {
                                cst_kind: "identifier",
                                name: "x",
                            },
                        ),
                    ),
                    definition: Identifier(
                        UnquotedIdentifier(
                            UnquotedIdentifier {
                                cst_kind: "identifier",
                                name: "a",
                            },
                        ),
                    ),
                },
            ),
        ],
    }            
            "#]],
        );
    }
}