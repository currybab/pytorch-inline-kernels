(identifier) @variable

(field_identifier) @property

(namespace_identifier) @namespace

(concept_definition
  name: (identifier) @concept)

(requires_clause
  constraint: (template_type
    name: (type_identifier) @concept))

(call_expression
  function: (qualified_identifier
    name: (identifier) @function))

(call_expression
  (qualified_identifier
    (identifier) @function.call))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (identifier) @function.call)))

(call_expression
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (identifier) @function.call))))

((qualified_identifier
  (qualified_identifier
    (qualified_identifier
      (qualified_identifier
        (identifier) @function.call)))) @_parent
  (#has-ancestor? @_parent call_expression))

(call_expression
  function: (identifier) @function)

(call_expression
  function: (field_expression
    field: (field_identifier) @function))

(preproc_function_def
  name: (identifier) @function.special)

(template_function
  name: (identifier) @function)

(template_method
  name: (field_identifier) @function)

(function_declarator
  declarator: (identifier) @function)

(function_declarator
  declarator: (qualified_identifier
    name: (identifier) @function))

(function_declarator
  declarator: (field_identifier) @function)

(operator_name
  (identifier)? @operator) @function

(operator_name
  "<=>" @operator.spaceship)

(destructor_name
  (identifier) @function)

((namespace_identifier) @type
  (#match? @type "^[A-Z]"))

(auto) @type

(type_identifier) @type

type: (primitive_type) @type.builtin

(sized_type_specifier) @type.builtin

; GNU __attribute__
(attribute_specifier) @attribute

(attribute_specifier
  (argument_list
    (identifier) @attribute))

; C++11 [[attributes]]
(attribute
  prefix: (identifier) @attribute)

(attribute
  name: (identifier) @attribute)

((identifier) @constant.builtin
  (#match? @constant.builtin "^_*[A-Z][A-Z\\d_]*$"))

(statement_identifier) @label

(this) @variable.builtin

"static_assert" @function.builtin

[
  "alignas"
  "alignof"
  "class"
  "concept"
  "consteval"
  "constexpr"
  "constinit"
  "decltype"
  "delete"
  "enum"
  "explicit"
  "extern"
  "final"
  "friend"
  "inline"
  "namespace"
  "new"
  "noexcept"
  "operator"
  "override"
  "private"
  "protected"
  "public"
  "requires"
  "sizeof"
  "struct"
  "template"
  "thread_local"
  "typedef"
  "typename"
  "union"
  "using"
  "virtual"
  (storage_class_specifier)
  (type_qualifier)
] @keyword

[
  "break"
  "case"
  "catch"
  "co_await"
  "co_return"
  "co_yield"
  "continue"
  "default"
  "do"
  "else"
  "for"
  "goto"
  "if"
  "return"
  "switch"
  "throw"
  "try"
  "while"
] @keyword.control

[
  "#define"
  "#elif"
  "#elifdef"
  "#elifndef"
  "#else"
  "#endif"
  "#if"
  "#ifdef"
  "#ifndef"
  "#include"
  (preproc_directive)
] @keyword.preproc @preproc

(comment) @comment

[
  (true)
  (false)
] @boolean

[
  (null)
  "nullptr"
] @constant.builtin

(number_literal) @number

[
  (string_literal)
  (system_lib_string)
  (char_literal)
  (raw_string_literal)
] @string

(escape_sequence) @string.escape

[
  ","
  ":"
  "::"
  ";"
  (raw_string_delimiter)
] @punctuation.delimiter

[
  "{"
  "}"
  "("
  ")"
  "["
  "]"
] @punctuation.bracket

[
  "."
  ".*"
  "->*"
  "~"
  "-"
  "--"
  "-="
  "->"
  "="
  "!"
  "!="
  "|"
  "|="
  "||"
  "^"
  "^="
  "&"
  "&="
  "&&"
  "+"
  "++"
  "+="
  "*"
  "*="
  "/"
  "/="
  "%"
  "%="
  "<<"
  "<<="
  ">>"
  ">>="
  "<"
  "=="
  ">"
  "<="
  ">="
  "?"
  "and"
  "and_eq"
  "bitand"
  "bitor"
  "compl"
  "not"
  "not_eq"
  "or"
  "or_eq"
  "xor"
  "xor_eq"
] @operator

"<=>" @operator.spaceship

(binary_expression
  operator: "<=>" @operator.spaceship)

(conditional_expression
  ":" @operator)

(user_defined_literal
  (literal_suffix) @operator)

; CUDA ------------------------------------------------------------------------
; Everything above is Zed's C++ highlights.scm (minus C++20 module nodes, which
; tree-sitter-cuda does not have). Patterns below come later and win.

[
  "__host__"
  "__device__"
  "__global__"
  "__managed__"
  "__forceinline__"
  "__noinline__"
  "__shared__"
  "__local__"
  "__constant__"
  "__grid_constant__"
  "__restrict__"
  "__launch_bounds__"
] @keyword.modifier

(kernel_call_syntax
  [
    "<<<"
    ">>>"
  ] @punctuation.special)

(call_expression
  function: (identifier) @function.kernel
  (kernel_call_syntax))

(call_expression
  function: (template_function
    name: (identifier) @function.kernel)
  (kernel_call_syntax))

((identifier) @variable.special
  (#any-of? @variable.special "threadIdx" "blockIdx" "blockDim" "gridDim" "warpSize"))

((identifier) @function.builtin
  (#match? @function.builtin "^(__syncthreads(_count|_and|_or)?|__syncwarp|__threadfence(_block|_system)?|__(shfl|shfl_up|shfl_down|shfl_xor)_sync|__(any|all|ballot|activemask|match_any|match_all)(_sync)?|atomic(Add|Sub|Exch|Min|Max|Inc|Dec|CAS|And|Or|Xor)(_block|_system)?|__ldg|__ldcg|__ldca|__ldcs|__stcg|__stcs|__popc(ll)?|__clz(ll)?|__ffs(ll)?|__brev(ll)?|__fmaf?_r[nzud]|__f?(add|mul|div)_r[nzud]|__expf|__logf|__sinf|__cosf|__powf|__saturatef|__float2half|__half2float|cuda[A-Z][A-Za-z0-9]*)$"))
