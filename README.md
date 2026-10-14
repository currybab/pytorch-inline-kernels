# PyTorch load_inline for Zed

[pytorch-load-inline-highlighter](https://github.com/msaroufim/pytorch-load-inline-highlighter)(VS Code)를
Zed로 옮긴 확장입니다. `torch.utils.cpp_extension.load_inline`에 넘기는 C++/CUDA 소스 문자열을
C++/CUDA 문법으로 하이라이트하고, `.cu`/`.cuh`용 CUDA 언어도 함께 제공합니다.

## 왜 별도 언어("Python (load_inline)")인가

VS Code는 TextMate injection으로 기존 Python 문법에 규칙을 끼워 넣을 수 있지만, Zed는 언어
주입 규칙이 각 언어의 `injections.scm` 안에 있고 **확장이 내장 Python 언어를 덮어쓸 수 없습니다**
(같은 이름으로 등록하면 거부됨). 그래서 이 확장은 Zed 내장 Python의 쿼리를 그대로 가져온
`Python (load_inline)` 언어를 따로 만들고, 거기에 load_inline 규칙을 추가했습니다.

내장 Python 언어 서버(basedpyright, ruff)는 "Python" 언어에만 붙기 때문에, 이 확장이
같은 서버를 `Python (load_inline)`용으로 직접 띄웁니다.

| 서버 id | 실행 파일 | 없을 때 |
| --- | --- | --- |
| `load-inline-basedpyright` | PATH의 `basedpyright-langserver` | npm으로 `basedpyright` 설치 |
| `load-inline-ruff` | PATH의 `ruff` (`ruff server`) | GitHub 최신 릴리스 다운로드 |

실행 파일을 직접 지정하려면 `"lsp": { "load-inline-ruff": { "binary": { "path": "...", "arguments": ["server"] } } }`
처럼 `binary.path`를 쓰면 위 탐색보다 우선합니다.

`lsp.load-inline-basedpyright` / `lsp.load-inline-ruff` 설정이 없으면 Zed의
`lsp.basedpyright` / `lsp.ruff` 설정을 그대로 씁니다. 워크트리 루트의 `.venv`/`venv`를
자동으로 인터프리터로 잡습니다(직접 지정한 값이 있으면 그쪽이 우선).

툴체인 선택기, pytest 실행 버튼(runnables), 디버거 설정 같은 내장 Python 전용 기능은
`Python (load_inline)`에서는 동작하지 않습니다.

## 설치 (dev extension)

1. **rustup의 Rust가 PATH에서 Homebrew Rust보다 앞에 있어야 합니다.** Zed는 dev extension을
   `cargo build --target wasm32-wasip2`로 빌드하는데, Homebrew `rust`에는 이 타깃이 없습니다.
   예: `~/.zshrc` 맨 끝에 `export PATH="$HOME/.cargo/bin:$PATH"` 추가(또는 `brew uninstall rust`)
   후 Zed를 완전히 종료했다가 다시 실행.
2. Zed 명령 팔레트 → **zed: install dev extension** → 이 디렉터리 선택.
3. `examples/load_inline_example.py`, `examples/vector_add.cu`를 열어 확인.

빌드 실패 로그는 명령 팔레트의 **zed: open log**에서 볼 수 있습니다.

## 파일을 이 언어로 열기

`*.py`는 기본적으로 내장 Python이 계속 담당합니다. 원하는 파일만 골라서 쓰세요.

- **파일별(modeline):** 파일 첫 줄이나 마지막 줄에
  ```python
  # vim: set ft=load_inline:
  ```
- **경로별(settings.json):**
  ```jsonc
  {
    "file_types": {
      "Python (load_inline)": ["**/kernels/**/*.py", "*_ext.py"],
      // Zed 내장 C++도 .cu/.cuh를 주장하므로, CUDA 언어로 확실히 열려면:
      "CUDA": ["cu", "cuh"]
    }
  }
  ```
- **수동:** 상태 표시줄의 언어 선택기에서 `Python (load_inline)`.

포매터/import 정리를 내장 Python처럼 쓰려면:

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

## 감지 규칙

`languages/python-load-inline/injections.scm`. 한 문자열에는 최대 한 번만 주입되도록 서로
배타적으로 작성했습니다(Zed는 패턴 간 중복 주입을 합치지 않음). 우선순위:

1. **키워드 인자:** `cpp_sources=` → C++, `cuda_sources=` / `hip_sources=` → CUDA
   (문자열, 문자열 list/tuple 모두)
2. **변수 이름:** `cuda_*` `hip_*` `kernel_code` `*_cuda` `*_cu` `*_hip` `*_kernel` → CUDA,
   `cpp_*` `*_cpp` → C++ (접두사가 접미사보다 우선: `cpp_kernel`은 C++)
3. **내용:** `__global__`/`__device__`/`__host__`/`threadIdx`/`blockIdx`/... → CUDA,
   `torch::Tensor`/`#include <torch/...>` → C++ (대입문만; docstring·함수 인자는 제외)
4. **마커 주석:** 대입문 바로 윗줄의 `# cuda`, `# cpp`, `# c++`, `# language=cuda`

f-string(`f"""... {BLOCK} ..."""`)은 보간 사이의 조각들을 하나의 C++ 문서로 파싱합니다.
HIP 소스는 CUDA 문법으로 칠합니다.

## 개발

```sh
scripts/check.sh                                   # 쿼리 테스트 + Rust 테스트 + clippy
ZED_EXTENSION_CLI=/path/to/zed-extension scripts/check.sh   # Zed 공식 CLI 검증까지
```

- 쿼리 테스트(`tests/`, pytest): 모든 `.scm`이 실제 문법(tree-sitter-python 0.25, `extension.toml`에
  고정된 tree-sitter-cuda)에서 컴파일되는지, `tests/fixtures/injections.py`의 `EXPECT:<lang>` 표시대로
  정확히 한 번씩 주입되는지 확인합니다.
- `zed-extension`은 Zed 저장소에서 `cargo build --release -p extension_cli`로 빌드합니다.
- `extension.toml`에 `[grammars.python]`을 **넣지 마세요.** 확장이 언로드될 때 Zed가 그 이름의
  문법을 지워서 내장 Python 문법까지 사라집니다.

## 출처와 라이선스

- `languages/python-load-inline/*.scm`, `languages/cuda/*.scm`, 두 `config.toml`은 Zed
  v1.2.6의 `crates/grammars/src/{python,cpp}`에서 가져와 수정했습니다(GPL-3.0-or-later).
  그래서 이 확장도 GPL-3.0-or-later입니다(`LICENSE`).
- CUDA 문법: [tree-sitter-grammars/tree-sitter-cuda](https://github.com/tree-sitter-grammars/tree-sitter-cuda) (MIT).
- 감지 규칙은 [msaroufim/pytorch-load-inline-highlighter](https://github.com/msaroufim/pytorch-load-inline-highlighter)를 따랐습니다.
