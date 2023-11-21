/*
2 Variables
	2.1 Zero/One Variables (Done)
	2.2 Integer Variables (Done)
	2.3 Symbolic Variables (Done)
	2.4 Real Variables (Done)
	2.5 Set Variables (Done)
	2.6 Graph Variables (Done)
	2.7 Stochastic Variables TODO
	2.8 Qualitative Variable TODO
	2.9 Arrays of Variables 
	2.9.1 Using Compact Forms
	2.9.2 Dealing With Mixed Domains
	2.10 Empty Domains, Undefined and Useless Variables
	2.11 Solutions
3 Objectives
	3.1 Objectives in Functional Formss
	3.2 Objectives in Specialized Formss
	3.3 Multi-objective Optimizatios

*/

PREC = {
	call: 5,
	negative: 4,
	range: 3,
}

MAX_PREC = Math.max(...Object.values(PREC))

module.exports = grammar({
	name: "xcps3",

	extras: ($) => [/\s/, $.block_comment, $.note_comment],

	word: ($) => $.identifier,

	rules: {
		source_file: ($) => 
			seq(
				optional(
					element("instance", $.attribute, repeat(field("item", $.element)))
				),
				optional(
					element("instantiation", $.attribute, repeat(field("item", $.element)))
				)
			),

		// instantiation: ($) => 
		// 	seq(
		// 		element("list", "", repeat(field("names", $._expression))),
		// 		element("value", "", repeat(field("definitions", $._expression))),
		// 	),

		element: ($) => 
			element(
				$.identifier, 
				$.attribute, 
				repeat(field("children", choice(
					$.element, 
					prec.left(field("definition", repeat1($._expression)))
				)))
			),

		attribute: ($) => 
			seq(
				field("name", $.identifier),
				optional(seq('=', sepBy1(" ", field("value", $._quoted_identifier)))),
		  	),
		
		_expression: ($) => 
			choice(
				$.identifier,
				$.boolean_literal,
				$.integer_literal,
				$.decimal_literal,
				$.rational_literal,
				$.interval,
				$.infinity,
				$.parameter,
				$.indexed_access,
				$.call,
				$.set_constructor,
				$.tuple
			),

		indexed_access: ($) =>
			prec(
				PREC.call,
				seq(
					field("collection", $._expression),
					"[",
					sepBy(
						"][",
						optional(field("index", $._expression))
					),
					"]"
				)
			),
			

		call: ($) =>
			choice(
				prec(
					PREC.call,
					seq(
						field("name", $.identifier), 
						"(",
						optional(sepBy(",", field("argument", $._expression))),
						")",
					)
				),
				prec(
					PREC.negative,
					seq(field("name", "-"), field("argument", $._expression)),
				)
			),

		set_constructor: ($,) =>
			prec.left(
				PREC.range,
				seq(
					field("left", $._expression),
					field("operator", ".."),
					field("right", $._expression)
				)
			),
			
		interval: ($) =>
			seq(
				choice("[", "]"), // Infinity is written ]-infinity, infinity[
				field("left", $._expression),
				",",
				field("right", $._expression),
				choice("[", "]"),
			),
		
		tuple: ($) =>
			seq(
				"(",
				sepBy1(",", field("member", $._expression)),
				")"
			),

		// Positive is optionally removed as + as a prefix isn't supported in MZN
		boolean_literal : (_) => choice("true", "false"),
		integer_literal : (_) => seq(optional("+"), token(/[0-9]+/)),
		decimal_literal : (_) => seq(optional("+"), token(/[0-9]+\.[0-9]+/)),
		rational_literal : (_) => seq(optional("+"), token(/[0-9]+\/[0-9]+/)),
		infinity: (_) => seq(optional("+"), choice(/infinity/, "∞")),
		parameter: (_) => seq("%", token(/(\.\.\.|[0-9]+)/)),
		
		// As the language is XML technically there should be both a identifier and tag identifier
		// However, for simpicity we will just use the identifier for both
		identifier: (_) => /[A-Za-z][A-Za-z0-9_]*/,
		_quoted_identifier: ($) => choice(
			seq("'", choice(repeat1($.identifier), $.indexed_access, $._size_literal), "'"),
			seq('"', choice(repeat1($.identifier), $.indexed_access, $._size_literal), '"'),
		),
		// Probably will needed to be removed
		_size_literal: ($) => seq("[", $.integer_literal, "]"),

		block_comment: (_) => token(/<!--[^\!-<>]*-->/),
		note_comment: (_) => 
			seq(
				"note",
				"=",
				choice(
					seq("'", token(/[^']*/), "'"),
					seq('"', token(/[^"]*/), '"'),
				)
			),
	},
})

function sepBy(sep, rule) {
	return seq(repeat(seq(rule, sep)), optional(rule))
}

function sepBy1(sep, rule) {
	return seq(rule, repeat(seq(sep, rule)), optional(sep))
}

function element(name, attribute, contents) {
	return choice (
		// Normal tags
		seq(
			"<", 
			field("name", name),
			optional(repeat(field("attribute", attribute))),
			">", 
			contents, 
			"</", 
			name, 
			">"
		),
		// Self closing tags
		seq(
			"<", 
			name,
			optional(repeat(field("attribute", attribute))),
			"/>"
		)
	)
}
