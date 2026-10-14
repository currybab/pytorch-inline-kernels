((comment) @injection.content
  (#set! injection.language "comment"))

; SQL -----------------------------------------------------------------------------
([
  ; function calls
  (call
    [
      (attribute
        attribute: (identifier))
      (identifier)
    ]
    arguments: (argument_list
      (comment) @_comment
      (string
        (string_content) @injection.content)))
  ; string variables
  ((comment) @_comment
    .
    (expression_statement
      (assignment
        right: (string
          (string_content) @injection.content))))
]
  (#match? @_comment "^(#|#\\s+)(?i:sql)\\s*$")
  (#set! injection.language "sql"))

; load_inline ---------------------------------------------------------------------
; C++/CUDA sources embedded in Python strings, for
; torch.utils.cpp_extension.load_inline. HIP sources are highlighted as CUDA.
;
; The rules are mutually exclusive: Zed creates one syntax layer per match and
; does not deduplicate across patterns. Precedence, highest first:
;   1. keyword argument name   cpp_sources= / cuda_sources= / hip_sources=
;   2. variable name           cuda_* hip_* kernel_code *_cuda *_cu *_hip *_kernel -> cuda
;                              cpp_* *_cpp                                        -> cpp
;                              (case-insensitive; a cuda_/hip_/cpp_ prefix beats any suffix)
;   3. content                 __global__, threadIdx, ...   -> cuda
;                              torch::Tensor, <torch/...>   -> cpp
;   4. marker comment          `# cuda`, `# cpp`, `# language=cuda` on the line above
;
; Rules 2-4 accept the same value forms, the `right:` alternation repeated in
; each of them (tests/test_injections.py checks that the copies are identical):
; a string, implicitly concatenated strings, dedent/cleandoc of one string, and
; .strip()/.lstrip()/.rstrip() of either. Concatenated pieces form one
; injection. Predicates on @_wrap/@_strip only constrain the branch that
; captures them: a predicate on a capture with no nodes passes.
;
; Content checks use `@_str`, the whole string node, never the quantified
; `@injection.content`: Zed's Rust tree-sitter (0.26.8) returns true from
; `#any-match?` when no node matches, so the content rules matched every
; string. tests/query_runner runs the fixture on that engine.
;
; `((string_content) (interpolation)*)+` puts every literal segment of an
; f-string into a single injection, so `f"""... {BLOCK} ..."""` kernels parse as
; one C++ document instead of one broken layer per segment. Both engines agree
; on this form; `[(string_content) (interpolation)]+` inside an alternation
; yields one match per segment in Rust tree-sitter, and `(string_content)+`
; stops at the first interpolation in py-tree-sitter.
;
; Known limit: a trailing `x = 1  # cuda` also marks the next line's string.
; Queries cannot tell a trailing comment from one on its own line.

; 1. keyword arguments: a string, or a list/tuple of strings.
(keyword_argument
  name: (identifier) @_kw
  value: [
    (string ((string_content) @injection.content (interpolation)*)+)
    (list (string ((string_content) @injection.content (interpolation)*)+))
    (tuple (string ((string_content) @injection.content (interpolation)*)+))
  ]
  (#match? @_kw "^(cuda|hip)_sources$")
  (#set! injection.language "cuda"))

(keyword_argument
  name: (identifier) @_kw
  value: [
    (string ((string_content) @injection.content (interpolation)*)+)
    (list (string ((string_content) @injection.content (interpolation)*)+))
    (tuple (string ((string_content) @injection.content (interpolation)*)+))
  ]
  (#eq? @_kw "cpp_sources")
  (#set! injection.language "cpp"))

; 2. variable names (plain or attribute targets, annotated or not).
(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: [
    ; "..."
    (string ((string_content) @injection.content (interpolation)*)+) @_str
    ; "..." "..."  and  ("..." "...")
    (concatenated_string
      (string ((string_content) @injection.content (interpolation)*)+)+) @_str
    (parenthesized_expression
      (concatenated_string
        (string ((string_content) @injection.content (interpolation)*)+)+) @_str)
    ; dedent("...")  textwrap.dedent("...")  inspect.cleandoc("...")
    (call
      function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
      arguments: (argument_list
        .
        (string ((string_content) @injection.content (interpolation)*)+) @_str
        .)
      (#any-of? @_wrap "dedent" "cleandoc"))
    ; "...".strip()
    (call
      function: (attribute
        object: (string ((string_content) @injection.content (interpolation)*)+) @_str
        attribute: (identifier) @_strip)
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
    ; textwrap.dedent("...").strip()
    (call
      function: (attribute
        object: (call
          function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
          arguments: (argument_list
            .
            (string ((string_content) @injection.content (interpolation)*)+) @_str
            .))
        attribute: (identifier) @_strip)
      (#any-of? @_wrap "dedent" "cleandoc")
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
  ]
  (#match? @_name "(?i)^(cuda|hip)_|^kernel_code$|_(cuda|cu|hip|kernel)$")
  (#not-match? @_name "(?i)^cpp_")
  (#set! injection.language "cuda"))

(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: [
    ; "..."
    (string ((string_content) @injection.content (interpolation)*)+) @_str
    ; "..." "..."  and  ("..." "...")
    (concatenated_string
      (string ((string_content) @injection.content (interpolation)*)+)+) @_str
    (parenthesized_expression
      (concatenated_string
        (string ((string_content) @injection.content (interpolation)*)+)+) @_str)
    ; dedent("...")  textwrap.dedent("...")  inspect.cleandoc("...")
    (call
      function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
      arguments: (argument_list
        .
        (string ((string_content) @injection.content (interpolation)*)+) @_str
        .)
      (#any-of? @_wrap "dedent" "cleandoc"))
    ; "...".strip()
    (call
      function: (attribute
        object: (string ((string_content) @injection.content (interpolation)*)+) @_str
        attribute: (identifier) @_strip)
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
    ; textwrap.dedent("...").strip()
    (call
      function: (attribute
        object: (call
          function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
          arguments: (argument_list
            .
            (string ((string_content) @injection.content (interpolation)*)+) @_str
            .))
        attribute: (identifier) @_strip)
      (#any-of? @_wrap "dedent" "cleandoc")
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
  ]
  (#match? @_name "(?i)^cpp_|_cpp$")
  (#not-match? @_name "(?i)^(cuda|hip)_")
  (#set! injection.language "cpp"))

; 3. content, for assignments whose name did not decide already.
(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: [
    ; "..."
    (string ((string_content) @injection.content (interpolation)*)+) @_str
    ; "..." "..."  and  ("..." "...")
    (concatenated_string
      (string ((string_content) @injection.content (interpolation)*)+)+) @_str
    (parenthesized_expression
      (concatenated_string
        (string ((string_content) @injection.content (interpolation)*)+)+) @_str)
    ; dedent("...")  textwrap.dedent("...")  inspect.cleandoc("...")
    (call
      function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
      arguments: (argument_list
        .
        (string ((string_content) @injection.content (interpolation)*)+) @_str
        .)
      (#any-of? @_wrap "dedent" "cleandoc"))
    ; "...".strip()
    (call
      function: (attribute
        object: (string ((string_content) @injection.content (interpolation)*)+) @_str
        attribute: (identifier) @_strip)
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
    ; textwrap.dedent("...").strip()
    (call
      function: (attribute
        object: (call
          function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
          arguments: (argument_list
            .
            (string ((string_content) @injection.content (interpolation)*)+) @_str
            .))
        attribute: (identifier) @_strip)
      (#any-of? @_wrap "dedent" "cleandoc")
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
  ]
  (#not-match? @_name "(?i)^(cuda|hip|cpp)_|^kernel_code$|_(cuda|cu|hip|kernel|cpp)$")
  (#match? @_str "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b")
  (#set! injection.language "cuda"))

(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: [
    ; "..."
    (string ((string_content) @injection.content (interpolation)*)+) @_str
    ; "..." "..."  and  ("..." "...")
    (concatenated_string
      (string ((string_content) @injection.content (interpolation)*)+)+) @_str
    (parenthesized_expression
      (concatenated_string
        (string ((string_content) @injection.content (interpolation)*)+)+) @_str)
    ; dedent("...")  textwrap.dedent("...")  inspect.cleandoc("...")
    (call
      function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
      arguments: (argument_list
        .
        (string ((string_content) @injection.content (interpolation)*)+) @_str
        .)
      (#any-of? @_wrap "dedent" "cleandoc"))
    ; "...".strip()
    (call
      function: (attribute
        object: (string ((string_content) @injection.content (interpolation)*)+) @_str
        attribute: (identifier) @_strip)
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
    ; textwrap.dedent("...").strip()
    (call
      function: (attribute
        object: (call
          function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
          arguments: (argument_list
            .
            (string ((string_content) @injection.content (interpolation)*)+) @_str
            .))
        attribute: (identifier) @_strip)
      (#any-of? @_wrap "dedent" "cleandoc")
      (#any-of? @_strip "strip" "lstrip" "rstrip"))
  ]
  (#not-match? @_name "(?i)^(cuda|hip|cpp)_|^kernel_code$|_(cuda|cu|hip|kernel|cpp)$")
  (#not-match? @_str "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b")
  (#match? @_str "torch::Tensor|#include\\s*<torch/")
  (#set! injection.language "cpp"))

; 4. marker comment directly above an assignment that nothing else claimed.
((comment) @_comment
  .
  (expression_statement
    (assignment
      left: [
        (identifier) @_name
        (attribute attribute: (identifier) @_name)
      ]
      right: [
        ; "..."
        (string ((string_content) @injection.content (interpolation)*)+) @_str
        ; "..." "..."  and  ("..." "...")
        (concatenated_string
          (string ((string_content) @injection.content (interpolation)*)+)+) @_str
        (parenthesized_expression
          (concatenated_string
            (string ((string_content) @injection.content (interpolation)*)+)+) @_str)
        ; dedent("...")  textwrap.dedent("...")  inspect.cleandoc("...")
        (call
          function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
          arguments: (argument_list
            .
            (string ((string_content) @injection.content (interpolation)*)+) @_str
            .)
          (#any-of? @_wrap "dedent" "cleandoc"))
        ; "...".strip()
        (call
          function: (attribute
            object: (string ((string_content) @injection.content (interpolation)*)+) @_str
            attribute: (identifier) @_strip)
          (#any-of? @_strip "strip" "lstrip" "rstrip"))
        ; textwrap.dedent("...").strip()
        (call
          function: (attribute
            object: (call
              function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
              arguments: (argument_list
                .
                (string ((string_content) @injection.content (interpolation)*)+) @_str
                .))
            attribute: (identifier) @_strip)
          (#any-of? @_wrap "dedent" "cleandoc")
          (#any-of? @_strip "strip" "lstrip" "rstrip"))
      ]))
  (#match? @_comment "^#\\s*((language|lang)\\s*[=:]\\s*)?(?i:cuda|hip)\\s*$")
  (#not-match? @_name "(?i)^(cuda|hip|cpp)_|^kernel_code$|_(cuda|cu|hip|kernel|cpp)$")
  (#not-match? @_str "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b|torch::Tensor|#include\\s*<torch/")
  (#set! injection.language "cuda"))

((comment) @_comment
  .
  (expression_statement
    (assignment
      left: [
        (identifier) @_name
        (attribute attribute: (identifier) @_name)
      ]
      right: [
        ; "..."
        (string ((string_content) @injection.content (interpolation)*)+) @_str
        ; "..." "..."  and  ("..." "...")
        (concatenated_string
          (string ((string_content) @injection.content (interpolation)*)+)+) @_str
        (parenthesized_expression
          (concatenated_string
            (string ((string_content) @injection.content (interpolation)*)+)+) @_str)
        ; dedent("...")  textwrap.dedent("...")  inspect.cleandoc("...")
        (call
          function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
          arguments: (argument_list
            .
            (string ((string_content) @injection.content (interpolation)*)+) @_str
            .)
          (#any-of? @_wrap "dedent" "cleandoc"))
        ; "...".strip()
        (call
          function: (attribute
            object: (string ((string_content) @injection.content (interpolation)*)+) @_str
            attribute: (identifier) @_strip)
          (#any-of? @_strip "strip" "lstrip" "rstrip"))
        ; textwrap.dedent("...").strip()
        (call
          function: (attribute
            object: (call
              function: [(identifier) @_wrap (attribute attribute: (identifier) @_wrap)]
              arguments: (argument_list
                .
                (string ((string_content) @injection.content (interpolation)*)+) @_str
                .))
            attribute: (identifier) @_strip)
          (#any-of? @_wrap "dedent" "cleandoc")
          (#any-of? @_strip "strip" "lstrip" "rstrip"))
      ]))
  (#match? @_comment "^#\\s*((language|lang)\\s*[=:]\\s*)?(?i:cpp|c\\+\\+)\\s*$")
  (#not-match? @_name "(?i)^(cuda|hip|cpp)_|^kernel_code$|_(cuda|cu|hip|kernel|cpp)$")
  (#not-match? @_str "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b|torch::Tensor|#include\\s*<torch/")
  (#set! injection.language "cpp"))
