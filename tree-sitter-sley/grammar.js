const PREC = {
  OR: 1,
  AND: 2,
  EQUAL: 3,
  COMPARE: 4,
  ADD: 5,
  MULTIPLY: 6,
  UNARY: 7,
  POSTFIX: 8,
  CALL: 9,
};

module.exports = grammar({
  name: 'sley',

  extras: $ => [
    /\s/,
    $.comment,
  ],

  word: $ => $.identifier,

  conflicts: $ => [
    [$.qualified_identifier, $.qualified_type_identifier],
  ],

  rules: {
    source_file: $ => repeat($._declaration),

    _declaration: $ => choice(
      $.module_declaration,
      $.import_declaration,
      $.type_declaration,
      $.effect_declaration,
      $.task_declaration,
      $.export_declaration,
      ';',
    ),

    module_declaration: $ => seq('module', field('name', $.qualified_identifier)),

    import_declaration: $ => seq(
      'import',
      field('module', $.qualified_identifier),
      optional(seq('as', field('alias', $.identifier))),
    ),

    export_declaration: $ => seq(
      'export',
      choice($.type_declaration, $.effect_declaration, $.task_declaration),
    ),

    type_declaration: $ => seq(
      'type',
      field('name', $.identifier),
      '=',
      field('value', $.type_expression),
    ),

    effect_declaration: $ => seq('effect', field('name', $.identifier)),

    task_declaration: $ => seq(
      'task',
      field('name', $.identifier),
      '->',
      field('return_type', $.type_expression),
      optional($.uses_clause),
      field('body', $.task_block),
    ),

    uses_clause: $ => seq(
      'uses',
      commaSep1(field('effect', $.qualified_identifier)),
    ),

    task_block: $ => seq(
      '{',
      repeat($.take_declaration),
      repeat($.statement),
      '}',
    ),

    take_declaration: $ => seq(
      'take',
      optional(field('qualifier', $.take_qualifier)),
      field('name', $.identifier),
      ':',
      field('type', $.type_expression),
    ),

    take_qualifier: _ => choice('gate', 'veil', 'taint', 'view'),

    type_expression: $ => choice(
      $.record_type,
      $.generic_type,
      $.qualified_identifier,
    ),

    generic_type: $ => seq(
      field('name', $.qualified_identifier),
      '<',
      commaSep1($.type_expression),
      '>',
    ),

    record_type: $ => seq(
      '{',
      repeat(seq(optional('slot'), field('field', $.record_type_field), optional(','))),
      '}',
    ),

    record_type_field: $ => seq(
      field('name', $.identifier),
      ':',
      field('type', $.type_expression),
    ),

    statement: $ => choice(
      $.binding_statement,
      $.set_statement,
      $.return_statement,
      $.if_statement,
      $.while_statement,
      $.for_statement,
      $.forge_statement,
      $.expression_statement,
    ),

    binding_statement: $ => seq(
      field('binding', $.binding_keyword),
      field('name', $.identifier),
      optional($.type_annotation),
      '=',
      field('value', $.expression),
    ),

    binding_keyword: _ => choice(
      'bind',
      'state',
      'cell',
      'knot',
      'slot',
      'gate',
      'lease',
      'veil',
      'dial',
      'flag',
      'memo',
      'cache',
      'derive',
      'flow',
      'port',
      'tally',
      'hole',
      'draft',
      'taint',
      'witness',
      'seal',
      'anchor',
      'view',
      'cursor',
    ),

    type_annotation: $ => seq(':', field('type', $.type_expression)),

    set_statement: $ => seq(
      'set',
      field('name', $.identifier),
      '=',
      field('value', $.expression),
    ),

    return_statement: $ => seq('return', field('value', $.expression)),

    if_statement: $ => seq(
      'if',
      field('condition', $.expression),
      field('then', $.block),
      optional(seq('else', field('else', $.block))),
    ),

    while_statement: $ => seq(
      'while',
      field('condition', $.expression),
      field('body', $.block),
    ),

    for_statement: $ => seq(
      choice('for', 'each'),
      field('item', $.identifier),
      'in',
      field('collection', $.expression),
      field('body', $.block),
    ),

    forge_statement: $ => seq('forge', field('body', $.block)),

    expression_statement: $ => field('value', $.expression),

    block: $ => seq('{', repeat($.statement), '}'),

    expression: $ => choice(
      $.if_expression,
      $.call_keyword_expression,
      $.binary_expression,
      $.unary_expression,
      $.try_expression,
      $.call_expression,
      $.field_expression,
      $.index_expression,
      $.record_literal,
      $.list_literal,
      $.map_literal,
      $.parenthesized_expression,
      $.string,
      $.number,
      $.boolean,
      $.qualified_type_identifier,
      $.qualified_identifier,
    ),

    if_expression: $ => prec.right(seq(
      'if',
      field('condition', $.expression),
      field('then', $.expression_block),
      'else',
      field('else', $.expression_block),
    )),

    expression_block: $ => prec(PREC.POSTFIX, seq('{', field('value', $.expression), '}')),

    call_keyword_expression: $ => prec(PREC.CALL, seq('call', field('target', $.expression))),

    binary_expression: $ => choice(
      binary($, '||', PREC.OR),
      binary($, '&&', PREC.AND),
      binary($, '==', PREC.EQUAL),
      binary($, '!=', PREC.EQUAL),
      binary($, '<', PREC.COMPARE),
      binary($, '<=', PREC.COMPARE),
      binary($, '>', PREC.COMPARE),
      binary($, '>=', PREC.COMPARE),
      binary($, '+', PREC.ADD),
      binary($, '-', PREC.ADD),
      binary($, '*', PREC.MULTIPLY),
      binary($, '/', PREC.MULTIPLY),
      binary($, '%', PREC.MULTIPLY),
    ),

    unary_expression: $ => prec(PREC.UNARY, seq(
      field('operator', choice('!', '-')),
      field('operand', $.expression),
    )),

    try_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('value', $.expression),
      '?',
    )),

    call_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('function', $.expression),
      '(',
      optional(commaSep1(field('argument', $.expression))),
      optional(','),
      ')',
    )),

    field_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('receiver', $.expression),
      '.',
      field('field', $.identifier),
    )),

    index_expression: $ => prec.left(PREC.POSTFIX, seq(
      field('collection', $.expression),
      '[',
      field('index', $.expression),
      ']',
    )),

    parenthesized_expression: $ => seq('(', $.expression, ')'),

    list_literal: $ => seq('[', optional(commaSep1($.expression)), optional(','), ']'),

    map_literal: $ => seq(
      'map',
      '{',
      optional(commaSep1($.map_entry)),
      optional(','),
      '}',
    ),

    map_entry: $ => seq(field('key', $.expression), ':', field('value', $.expression)),

    record_literal: $ => choice($.typed_record_literal, $.anonymous_record_literal),

    typed_record_literal: $ => prec(PREC.POSTFIX, seq(
      field('type', $.qualified_type_identifier),
      '{',
      optional(commaSep1($.record_field)),
      optional(','),
      '}',
    )),

    anonymous_record_literal: $ => seq(
      '{',
      optional(commaSep1($.record_field)),
      optional(','),
      '}',
    ),

    record_field: $ => seq(field('name', $.identifier), ':', field('value', $.expression)),

    qualified_identifier: $ => prec.left(seq($.identifier, repeat(seq('.', $.identifier)))),

    qualified_type_identifier: $ => prec.left(seq(
      repeat(seq($.identifier, '.')),
      $.type_identifier,
      repeat(seq('.', $.type_identifier)),
    )),

    type_identifier: _ => /[A-Z][A-Za-z0-9_]*/,

    boolean: _ => choice('true', 'false'),

    identifier: _ => /[A-Za-z_][A-Za-z0-9_]*/,

    number: _ => /[0-9]+(\.[0-9]+)?/,

    string: _ => /"([^"\\]|\\.)*"/,

    comment: _ => token(choice(seq('//', /[^\n]*/), seq('#', /[^\n]*/))),
  },
});

function binary($, operator, precedence) {
  return prec.left(precedence, seq(
    field('left', $.expression),
    field('operator', operator),
    field('right', $.expression),
  ));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}
