module.exports = grammar({
  name: "sley",

  extras: ($) => [/\s/, $.comment],

  word: ($) => $.identifier,

  rules: {
    source_file: ($) => repeat($._item),
    _item: ($) => choice(
      $.module_declaration,
      $.import_declaration,
      $.task_declaration,
      $.type_declaration,
      $.type_alias,
      $.effect_declaration,
      $.annotation,
    ),
    module_declaration: ($) => seq("module", $.qualified_identifier),
    import_declaration: ($) => seq("import", $.qualified_identifier, optional(seq("as", $.identifier))),
    annotation: ($) => seq("@", $.qualified_identifier, ":", $.string),
    task_declaration: ($) => seq(
      optional("export"),
      "task",
      $.identifier,
      optional($.parameter_list),
      "->",
      $.type_expression,
      optional(seq("uses", commaSep1($.qualified_identifier))),
      $.block,
    ),
    type_declaration: ($) => seq(optional("export"), "type", $.identifier, optional("="), "{", repeat($.slot_declaration), "}"),
    effect_declaration: ($) => seq(optional("export"), "effect", $.identifier),
    type_alias: ($) => seq(optional("export"), "type", $.identifier, "=", $.type_expression, optional(";")),
    slot_declaration: ($) => seq("slot", $.identifier, ":", $.type_expression),
    block: ($) => seq("{", repeat($._statement), "}"),
    _statement: ($) => choice(
      $.take_statement,
      $.binding_statement,
      $.set_statement,
      $.return_statement,
      $.expression_statement,
      $.if_statement,
      $.while_statement,
      $.each_statement,
    ),
    take_statement: ($) => seq("take", optional(choice("gate", "veil", "taint", "view")), $.identifier, ":", $.type_expression),
    binding_statement: ($) => seq(choice("bind", "state", "tally"), $.identifier, "=", $.expression),
    set_statement: ($) => seq("set", $.identifier, "=", $.expression),
    return_statement: ($) => seq("return", $.expression),
    expression_statement: ($) => $.expression,
    if_statement: ($) => seq("if", $.boolean_expression, $.block, optional(seq("else", $.block))),
    while_statement: ($) => seq("while", $.boolean_expression, $.block),
    each_statement: ($) => seq("each", $.identifier, "in", $.expression, $.block),
    expression: ($) => choice(
      $.call_expression,
      $.array_expression,
      $.grouped_expression,
      $.identifier,
      $.number,
      $.string,
      $.bool,
      $.field_expression,
      $.result_expression,
    ),
    boolean_expression: ($) => seq($.expression, optional(seq($.comparator, $.expression))),
    grouped_expression: ($) => seq("(", $.expression, ")"),
    array_expression: ($) => seq("[", commaSep($.expression), "]"),
    call_expression: ($) => seq("call", $.qualified_identifier, "(", optional(commaSep1($.expression)), ")", optional("?")),
    field_expression: ($) => seq($.identifier, ".", $.identifier),
    result_expression: ($) => seq(choice("Ok", "Err"), "(", optional($.expression), ")"),
    comparator: ($) => choice("==", "!=", "<=", ">=", "<", ">", "and", "or"),
    parameter_list: ($) => seq("(", optional(commaSep($.parameter)), ")"),
    parameter: ($) => seq($.identifier, ":", $.type_expression),
    type_expression: ($) => choice($.qualified_identifier, seq($.qualified_identifier, "<", commaSep1($.type_expression), ">")),
    qualified_identifier: ($) => seq($.identifier, repeat(seq(".", $.identifier))),
    identifier: (_) => /[A-Za-z_][A-Za-z0-9_]*/,
    number: (_) => /[0-9]+/,
    string: (_) => /"([^"\\]|\\.)*"/,
    bool: (_) => choice("true", "false"),
    comment: (_) => token(seq("//", /.*/)),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}
