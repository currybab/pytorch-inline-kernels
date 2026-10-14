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
;                              (a cuda_/hip_/cpp_ prefix beats any suffix)
;   3. content                 __global__, threadIdx, ...   -> cuda
;                              torch::Tensor, <torch/...>   -> cpp
;   4. marker comment          `# cuda`, `# cpp`, `# language=cuda` on the line above
;
; `[(string_content) (interpolation)]+` puts every literal segment of an f-string
; into a single injection, so `f"""... {BLOCK} ..."""` kernels parse as one C++
; document instead of one broken layer per segment.

; 1. keyword arguments: a string, or a list/tuple of strings.
(keyword_argument
  name: (identifier) @_kw
  value: [
    (string [(string_content) @injection.content (interpolation)]+)
    (list (string [(string_content) @injection.content (interpolation)]+))
    (tuple (string [(string_content) @injection.content (interpolation)]+))
  ]
  (#match? @_kw "^(cuda|hip)_sources$")
  (#set! injection.language "cuda"))

(keyword_argument
  name: (identifier) @_kw
  value: [
    (string [(string_content) @injection.content (interpolation)]+)
    (list (string [(string_content) @injection.content (interpolation)]+))
    (tuple (string [(string_content) @injection.content (interpolation)]+))
  ]
  (#eq? @_kw "cpp_sources")
  (#set! injection.language "cpp"))

; 2. variable names (plain or attribute targets, annotated or not).
(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: (string [(string_content) @injection.content (interpolation)]+)
  (#match? @_name "^((cuda|hip)_|kernel_code$)")
  (#set! injection.language "cuda"))

(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: (string [(string_content) @injection.content (interpolation)]+)
  (#match? @_name "_(cuda|cu|hip|kernel)$")
  (#not-match? @_name "^(cuda|hip|cpp)_")
  (#set! injection.language "cuda"))

(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: (string [(string_content) @injection.content (interpolation)]+)
  (#match? @_name "^cpp_")
  (#set! injection.language "cpp"))

(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: (string [(string_content) @injection.content (interpolation)]+)
  (#match? @_name "_cpp$")
  (#not-match? @_name "^(cuda|hip|cpp)_")
  (#set! injection.language "cpp"))

; 3. content, for assignments whose name did not decide already.
(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: (string [(string_content) @injection.content (interpolation)]+)
  (#not-match? @_name "^(cuda|hip|cpp)_|_(cuda|cu|hip|kernel|cpp)$|^kernel_code$")
  (#any-match? @injection.content "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b")
  (#set! injection.language "cuda"))

(assignment
  left: [
    (identifier) @_name
    (attribute attribute: (identifier) @_name)
  ]
  right: (string [(string_content) @injection.content (interpolation)]+)
  (#not-match? @_name "^(cuda|hip|cpp)_|_(cuda|cu|hip|kernel|cpp)$|^kernel_code$")
  (#not-match? @injection.content "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b")
  (#any-match? @injection.content "torch::Tensor|#include\\s*<torch/")
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
      right: (string [(string_content) @injection.content (interpolation)]+)))
  (#match? @_comment "^#\\s*((language|lang)\\s*[=:]\\s*)?(?i:cuda|hip)\\s*$")
  (#not-match? @_name "^(cuda|hip|cpp)_|_(cuda|cu|hip|kernel|cpp)$|^kernel_code$")
  (#not-match? @injection.content "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b|torch::Tensor|#include\\s*<torch/")
  (#set! injection.language "cuda"))

((comment) @_comment
  .
  (expression_statement
    (assignment
      left: [
        (identifier) @_name
        (attribute attribute: (identifier) @_name)
      ]
      right: (string [(string_content) @injection.content (interpolation)]+)))
  (#match? @_comment "^#\\s*((language|lang)\\s*[=:]\\s*)?(?i:cpp|c\\+\\+)\\s*$")
  (#not-match? @_name "^(cuda|hip|cpp)_|_(cuda|cu|hip|kernel|cpp)$|^kernel_code$")
  (#not-match? @injection.content "__(global|device|host)__|\\b(blockIdx|threadIdx|blockDim|gridDim)\\b|torch::Tensor|#include\\s*<torch/")
  (#set! injection.language "cpp"))
