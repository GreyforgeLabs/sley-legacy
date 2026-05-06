[
  "module"
  "import"
  "as"
  "export"
  "type"
  "effect"
  "task"
  "uses"
  "take"
  "return"
  "if"
  "else"
  "while"
  "for"
  "each"
  "in"
  "forge"
  "set"
  "call"
  "map"
] @keyword

(binding_keyword) @keyword
(take_qualifier) @keyword

(comment) @comment
(string) @string
(number) @number
(boolean) @constant.builtin

(module_declaration name: (qualified_identifier) @namespace)
(import_declaration module: (qualified_identifier) @namespace)
(type_declaration name: (identifier) @type)
(effect_declaration name: (identifier) @constant)
(task_declaration name: (identifier) @function)
(take_declaration name: (identifier) @variable.parameter)
(binding_statement name: (identifier) @variable)
(set_statement name: (identifier) @variable)
(record_type_field name: (identifier) @property)
(record_field name: (identifier) @property)

(call_expression function: (expression (qualified_identifier) @function.call))
(call_expression function: (expression (qualified_type_identifier) @function.call))
(call_keyword_expression
  target: (expression
    (call_expression function: (expression (qualified_identifier) @function.call))))
