# PyTorch Inline Kernels for Zed

Zed support for C++/CUDA kernels written inline in Python for PyTorch: the source strings you pass
to `torch.utils.cpp_extension.load_inline` are highlighted as C++/CUDA, with snippets for CUDA and
the PyTorch C++ extension API. It started as a port of the VS Code extension
[pytorch-load-inline-highlighter](https://github.com/msaroufim/pytorch-load-inline-highlighter).

The extension provides four things:

1. **`Python (load_inline)` language**: looks exactly like Zed's built-in Python, but highlights
   matching strings as C++/CUDA.
2. **`CUDA` language**: used for `.cu`/`.cuh` files and for injected CUDA strings.
3. **Snippets**: completion candidates for CUDA built-ins (`threadIdx`, `__syncthreads`,
   `atomicAdd`, ...) and PyTorch C++ extension idioms (`TORCH_CHECK`, `data_ptr`,
   `AT_DISPATCH_FLOATING_TYPES`, ...), inside CUDA/C++ strings as well as `.cu`/C++ files.
4. **Language servers**: attaches basedpyright and Ruff to `Python (load_inline)` files.

Installation, highlighting and language servers were verified on Zed 1.23.3 (macOS, Apple Silicon).

## Requirements

| Item | When needed | Notes |
| --- | --- | --- |
| Zed (with extension API 0.7.0) | Always | Verified on 1.23.3 |
| Rust installed via rustup | Only when installing or rebuilding | Homebrew `rust` cannot build it (see Install, step 1) |
| Network access | First install | Downloads wasi-sdk, tree-sitter-python, tree-sitter-cuda and, if needed, basedpyright/ruff |
| Node.js | When basedpyright is auto-installed | Zed uses `node` from PATH or downloads its own |
| NVIDIA GPU, CUDA Toolkit | **Not needed** | Highlighting only, so it works on macOS too |

## Install (dev extension)

1. **rustup's Rust must come before Homebrew's Rust on PATH.** Zed builds dev extensions with
   `cargo build --target wasm32-wasip2`. Homebrew's `rust` does not ship that target, so the build
   fails with `can't find crate for core`. Do one of the following:
   - Without changing your shell config: quit Zed completely, then launch it from a terminal:
     ```sh
     PATH="$HOME/.cargo/bin:$PATH" /Applications/Zed.app/Contents/MacOS/zed
     ```
     (Zed launched from a terminal keeps that PATH instead of reloading the login shell environment.)
   - Permanently: add `export PATH="$HOME/.cargo/bin:$PATH"` at the end of `~/.zshrc`, or run
     `brew uninstall rust`, then restart Zed.
2. Command palette → **zed: install dev extension** → select this directory. The first build takes
   20–30 seconds.
3. Open `examples/load_inline_example.py` and `examples/vector_add.cu` to check.

After the first install, Zed launched normally (Dock etc.) keeps working. Step 1 only matters again
when you **Rebuild** or reinstall the extension from the Extensions page. To remove it, use
**Uninstall** on the Extensions page (⌘⇧X). On Zed 1.20 or older, restart Zed afterwards (see
[Effect on built-in Python](#effect-on-built-in-python)).

## When it works

### 1. When a file uses `Python (load_inline)`

`*.py` files **keep using Zed's built-in Python by default**; this extension does not claim the
`py` extension. Use one of the following to opt in. The language name in the bottom-right of the
status bar shows which language a file uses.

| Method | Condition |
| --- | --- |
| Modeline | `# vim: set ft=load_inline:` within the **first or last 5 lines** of the file (`load-inline` and `torch-inline` also work, case-insensitive). Disabled when Zed's `modeline_lines` setting is 0. |
| `file_types` | Globs in `settings.json`, e.g. `"file_types": { "Python (load_inline)": ["**/kernels/**/*.py"] }`. Use `["py"]` for every `.py` file. |
| Manual | Pick `Python (load_inline)` in the status bar language selector (applies to that buffer only). |

The following **do not work** in `Python (load_inline)` files: the toolchain (interpreter)
selector, pytest run buttons (runnables) and Python debugger configuration. Zed wires those to the
built-in "Python" language only, and extensions have no way to attach them.

### 2. When a string is highlighted as C++/CUDA

Inside a `Python (load_inline)` file, the rules below are checked **top to bottom** and only the
first match applies, so a string is never highlighted twice.

| Priority | Rule | Highlighted as CUDA | Highlighted as C++ |
| --- | --- | --- | --- |
| 1 | Keyword argument name (in any call) | `cuda_sources=` | `cpp_sources=`, `sycl_sources=` |
| 2 | Assigned variable name (case-insensitive, so `CUDA_SRC` counts) | `cuda_*`, `hip_*`, `kernel_code`, `*_cuda`, `*_cu`, `*_hip`, `*_kernel` | `cpp_*`, `*_cpp` |
| 3 | Content of the assigned string | contains `__global__` `__device__` `__host__` `threadIdx` `blockIdx` `blockDim` `gridDim` | contains `torch::Tensor` or `#include <torch/...>` |
| 4 | Marker comment above the assignment | `# cuda`, `# hip`, `# language=cuda`, `# lang: cuda` | `# cpp`, `# c++`, `# language=cpp` |

- Rule 1 applies when the value is a string, or a list/tuple of strings. If you pass variables
  (`cuda_sources=[a_src, b_src]`), each variable is judged where it is defined, by rules 2–4.
- In rule 2 a prefix beats a suffix: `cpp_kernel` is C++, `cuda_wrapper_cpp` is CUDA. Only names
  are case-insensitive; the rule 3 keywords are matched exactly, as they are case-sensitive in C++.
- Rules 2–4 apply only to assignments of the form `name = ...`, `self.name = ...` or
  `name: str = ...`, where the value is one of:
  - a string: `"..."`;
  - implicitly concatenated strings, `"..." "..."` or the same split over lines inside
    parentheses, highlighted together as one document;
  - `dedent("...")`, `textwrap.dedent("...")` or `inspect.cleandoc("...")` with a single string;
  - `.strip()`, `.lstrip()` or `.rstrip()` on either of the above, e.g.
    `textwrap.dedent("""...""").strip()`.
- A rule 4 marker must sit directly above the assignment. Blank lines in between are fine; another
  comment in between is not. A trailing comment counts too: after `x = 1  # cuda`, the string on
  the next line is highlighted as CUDA, because queries cannot tell a trailing comment from one on
  its own line.
- Plain strings, `r"..."`, `b"..."` and triple-quoted strings all work.
- In f-strings (`f"""... {BLOCK} ..."""`), the `{...}` parts stay Python and the remaining pieces
  are highlighted together as one C++/CUDA document.
- HIP (AMD ROCm) sources, found by a `hip` variable name or a `# hip` marker, are highlighted with
  the CUDA grammar, which shares their syntax. HIP runtime APIs (`hipMalloc`, ...) are not
  highlighted as built-ins. `load_inline` itself has no `hip_sources=`: ROCm builds of PyTorch
  convert `cuda_sources` to HIP automatically.
- SYCL sources (`sycl_sources=`) are C++ and highlighted as such.

**Not highlighted** (verified examples):

| Code | Why | Instead |
| --- | --- | --- |
| `cuda_src += "..."` | `+=` is not covered by the assignment rules | Write it as one string |
| `cuda_src = build("...")`, `cuda_src = "...".format(...)` | Only `dedent`/`cleandoc` calls and `strip` methods are recognized | Assign the string first, then transform it |
| `cuda_sources=textwrap.dedent("...")` | Rule 1 takes strings and lists/tuples of strings only | Assign it to a variable first |
| `{"cuda": "__global__ ..."}`, `return "__global__ ..."` | Not an assignment | Assign it to a variable first |
| `load_inline("m", "torch::Tensor f();")` | Positional arguments have no name | Use a keyword argument or a variable |
| `load_inline(cuda_src="...")`, `hip_sources="..."` | Only `load_inline`'s own source arguments count (`hip_sources` is not one) | — |
| `a_src, b_src = "...", "..."` | Multiple targets in one assignment | One assignment per line |
| Docstrings, `print("__global__")` | Rule 3 only looks at assignments, to avoid false positives | — |

### 3. When `.cu`/`.cuh` files use the CUDA language

This extension's CUDA language registers the `cu`/`cuh` extensions. **Zed's built-in C++ registers
them too**, so they may open as C++ depending on your setup (they opened as CUDA in the verified
setup). To be sure:

```jsonc
{ "file_types": { "CUDA": ["cu", "cuh"] } }
```

A `// vim: set ft=cuda:` modeline also works. On top of C++ highlighting, the CUDA language
highlights qualifiers such as `__global__`, built-in variables such as `threadIdx`, intrinsics such
as `__syncthreads`, `atomicAdd` and `cuda*`, and `<<< >>>` kernel launches. No language server
(clangd etc.) is attached.

### 4. When the language servers run

| Condition | Details |
| --- | --- |
| The file's language is `Python (load_inline)` | Built-in Python files keep Zed's own servers. |
| The project is **trusted** | Servers do not start in Zed's Restricted Mode. Click **⚠ Restricted Mode** at the top left of the window to trust the project. |
| The executable can be found | Looked up in the order below. |

| Server id | 1) Settings | 2) PATH | 3) Auto-install |
| --- | --- | --- | --- |
| `load-inline-basedpyright` | `lsp.<id>.binary.path` | `basedpyright-langserver` | Latest `basedpyright` from npm |
| `load-inline-ruff` | `lsp.<id>.binary.path` | `ruff` (run as `ruff server`, needs ruff 0.5+) | Latest `astral-sh/ruff` GitHub release |

- PATH means **the PATH of a shell opened in the project folder** (including direnv, an activated
  venv and pyenv shims).
- Auto-installed copies live in
  `~/Library/Application Support/Zed/extensions/work/pytorch-inline-kernels/`. Once installed, the
  server also starts offline from that copy.
- Settings are read per field (`settings`, `initialization_options`, `binary`): from
  `lsp.load-inline-basedpyright` / `lsp.load-inline-ruff` when that field is set there, otherwise
  from Zed's `lsp.basedpyright` / `lsp.ruff`. So you can keep your existing Python setup and
  override a single field under the wrapper id. `binary` is used only when it has a `path`;
  `binary.arguments` then replaces the default arguments, and `binary.env` is layered over the
  shell environment.
- **Interpreter (venv):**
  - `.venv` at the project root is used automatically: basedpyright itself falls back to
    `<root>/.venv` when no interpreter is configured.
  - `venv/` at the project root is used only when your `.gitignore` does not exclude it. Zed does
    not index the contents of gitignored directories, so the extension cannot see it then.
  - If your settings contain any of `python`, `pythonPath`, `venvPath`, `venv` or
    `defaultInterpreterPath`, your value wins.
  - For any other location, set `venvPath`/`venv` in `pyrightconfig.json` or `pyproject.toml`,
    or `lsp.load-inline-basedpyright.settings.python.pythonPath`.
- basedpyright uses the same defaults as built-in Python: `typeCheckingMode: "standard"`, and import
  organizing is left to Ruff (`disableOrganizeImports`). Values in your settings take precedence.
- Not yet verified in SSH remote projects.

To format and organize imports like built-in Python:

```jsonc
{
  "languages": {
    "Python (load_inline)": {
      "formatter": { "language_server": { "name": "load-inline-ruff" } },
      "code_actions_on_format": { "source.organizeImports.ruff": true }
    }
  }
}
```

### 5. When snippet completions appear

Zed offers snippets for every language at the cursor, including an injected one, so the
completion menu shows them while you type inside a highlighted CUDA/C++ string (rule 2 above) or
in a `.cu`/C++ file. Inside a string that is *not* highlighted, they do not appear.

| Where the cursor is | Snippets offered |
| --- | --- |
| CUDA string, `.cu`/`.cuh` file | CUDA set + PyTorch C++ set |
| C++ string (`cpp_sources`, `cpp_*`, ...) | PyTorch C++ set |
| **Any other C++ file in Zed** (`.cpp`, `.h`, ...) | PyTorch C++ set, because Zed keys snippets by language |

Type the beginning of a prefix and pick from the menu; `Tab` moves between the placeholders.

| Set | Prefixes (excerpt) |
| --- | --- |
| CUDA | `threadIdx` `blockIdx` `blockDim` `gridDim` (insert `.x`, editable), `idx` (global index), `gsl` (grid-stride loop), `num_blocks`, `kernel`, `__device__`, `__shared__`, `launch`, `launch_stream`, `__syncthreads`, `__syncwarp`, `__shfl_*_sync`, `warp_reduce_sum`, `atomicAdd` / `Max` / `Min` / `Exch` / `CAS`, `__ldg`, `unroll` |
| PyTorch C++ | `include_torch`, `tensor_fn`, `TORCH_CHECK`, `check_cuda`, `check_contiguous`, `CHECK_INPUT`, `data_ptr`, `accessor`, `empty_like`, `zeros`, `AT_DISPATCH_FLOATING_TYPES(_AND_HALF)`, `current_stream`, `C10_CUDA_KERNEL_LAUNCH_CHECK`, `PYBIND11_MODULE`, `m_def` |

Snippets are a fixed list matched by prefix. They do not know your variables or types; that would
need a C++ language server (clangd) on the string contents, which Zed does not provide for injected
code.

## Troubleshooting

| Symptom | What to check |
| --- | --- |
| Build fails on install (`can't find crate for core`) | Homebrew Rust is picked up first → Install, step 1 |
| Strings look like plain Python strings | Is the status bar language `Python (load_inline)`? → When it works, 1. If so → the rules and the "Not highlighted" table in 2 |
| `.cu` opens as C++ | Add `"CUDA": ["cu", "cuh"]` to `file_types` |
| No diagnostics or hover | Restricted Mode? Do `load-inline-*` servers appear in **dev: open language server logs**? |
| New snippets, languages or servers missing after updating the extension | Zed caches each extension's manifest and does not re-read it on restart. Click **Rebuild** on the Extensions page (with the PATH from Install, step 1) |
| No snippet completions in a string | Is the string highlighted as CUDA/C++? Snippets follow the highlighting → When it works, 2 and 5 |
| basedpyright cannot resolve imports | Venv location → the interpreter item in When it works, 4 |
| Regular Python files lost highlighting after uninstalling (Zed 1.20 or older) | Restart Zed → [Effect on built-in Python](#effect-on-built-in-python) |
| Anything else | Command palette → **zed: open log** |

## Why a separate language

VS Code lets an extension inject rules into the existing Python grammar through TextMate injection.
In Zed, injection rules live in each language's own `injections.scm`, and **an extension cannot
override the built-in Python language** (registering the same name is rejected). So this extension
copies Zed's built-in Python queries into a separate `Python (load_inline)` language and adds its
rules there. Zed's Python language servers are also bound to "Python" only, so the extension starts
the same servers for its own language.

### Effect on built-in Python

`Python (load_inline)` parses with tree-sitter-python, and Zed requires an extension to declare
every grammar its languages use, so `extension.toml` declares `[grammars.python]`. Zed keys
grammars by name:

- **Zed 1.21 and later** keep their native `python` grammar and skip the extension's copy (the log
  shows `not registering extension grammar python`). Built-in Python is unaffected, and both
  languages use the native grammar.
- **Zed 1.20 and older** replace the native grammar with the extension's copy while it is
  installed. The copy is pinned to the release Zed links (tree-sitter-python 0.25.0), so parsing is
  the same, but it runs as WebAssembly. Uninstalling removes the `python` grammar from the running
  Zed, so regular Python files lose highlighting until Zed restarts.

## Development

```sh
scripts/check.sh                                            # query tests + Rust tests + clippy
ZED_EXTENSION_CLI=/path/to/zed-extension scripts/check.sh   # plus validation with Zed's extension CLI
```

- Query tests (`tests/`, pytest) check that every `.scm` compiles against the real grammars
  (built from the tree-sitter-python and tree-sitter-cuda revisions pinned in `extension.toml`), and that
  strings in `tests/fixtures/injections.py` are injected exactly once, as their `EXPECT:<lang>`
  markers say. When changing a rule, add an example to the fixture first.
- The injection fixture runs on two engines: py-tree-sitter and `tests/query_runner`, a small Rust
  binary on the same `tree-sitter` crate version Zed links (0.27.0 in Zed 1.23.3; built with
  rustup's cargo on first run). They can disagree: text predicates on a quantified capture work in py-tree-sitter but had
  no effect in Zed, so trust the Rust result.
- Snippets: edit `snippets/src/cuda.json` or `snippets/src/torch.json`, then run
  `python3 scripts/build_snippets.py` to regenerate `snippets/cuda.json` and `snippets/c++.json`
  (Zed keys snippet files by language name). `tests/test_snippets.py` fails when they are stale.
- Build `zed-extension` from the Zed repository with `cargo build --release -p extension_cli`.
- **Keep the pinned grammars and tree-sitter in step with Zed** (see
  [Effect on built-in Python](#effect-on-built-in-python)). When Zed moves to a newer
  tree-sitter or tree-sitter-python, update `[grammars.python]`, `tests/query_runner/Cargo.toml`
  and the versions in `tests/test_manifest.py` together. The Python query files are copies of
  Zed's (`crates/grammars/src/python`); re-copy them when Zed changes them.

## Credits and inspiration

This project builds directly on the following work.

- **[msaroufim/pytorch-load-inline-highlighter](https://github.com/msaroufim/pytorch-load-inline-highlighter)**:
  the VS Code extension this started from. The idea and the detection rules (keyword arguments,
  variable names, content keywords, HIP as CUDA) come from it, except `hip_sources=`, which
  `load_inline` does not accept.
- **[Zed](https://github.com/zed-industries/zed)** (v1.23.3):
  - The Python and C++ query files and language configs are adapted from
    `crates/grammars/src/{python,cpp}`.
  - The basedpyright/Ruff wrappers mirror Zed's built-in Python adapters
    (`crates/languages/src/python.rs`): launch arguments, workspace configuration keys and defaults.
  - Reading the language registry, syntax map, extension host and snippet provider sources shaped
    the design (why a separate language, what declaring the `python` grammar does to built-in
    Python, how injections and snippets are resolved).
  - The `zed_extension_api` crate and the `extension_cli` tool are used to build and validate the
    extension.
- **[d3banjan/zed-cuda](https://github.com/d3banjan/zed-cuda)**: an existing CUDA extension for
  Zed. Its pinned tree-sitter-cuda revision and its CUDA highlight captures were the starting point
  for the bundled CUDA language.
- **[tree-sitter-grammars/tree-sitter-cuda](https://github.com/tree-sitter-grammars/tree-sitter-cuda)**:
  the CUDA grammar, including its own CUDA highlight query.
- **[tree-sitter](https://github.com/tree-sitter/tree-sitter)**,
  **[tree-sitter-python](https://github.com/tree-sitter/tree-sitter-python)** and
  **[py-tree-sitter](https://github.com/tree-sitter/py-tree-sitter)**: parsing, and the query tests.
- **[basedpyright](https://github.com/DetachHead/basedpyright)** and
  **[Ruff](https://github.com/astral-sh/ruff)**: the language servers this extension starts.
- **[PyTorch](https://github.com/pytorch/pytorch)**: `torch.utils.cpp_extension.load_inline` and the
  C++ extension API (`TORCH_CHECK`, `AT_DISPATCH_*`, `PYBIND11_MODULE`, ...) that the highlighting
  and snippets target.

## License

GPL-3.0-or-later (`LICENSE`), because the query files and language configs are adapted from Zed's
GPL-3.0-or-later sources. The grammars (tree-sitter-python and tree-sitter-cuda) are MIT and are
fetched at build time, not vendored.
