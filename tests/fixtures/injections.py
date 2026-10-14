# Injection fixture for languages/python-load-inline/injections.scm.
#
# Every string literal that contains an `EXPECT:<lang>` marker must be injected
# exactly once with that language. `EXPECT:none` means the string must not be
# injected at all. Strings without a marker are not checked.
from torch.utils.cpp_extension import load_inline

# --- load_inline keyword arguments ------------------------------------------

load_inline(
    name="kw_string",
    cpp_sources="""// EXPECT:cpp
torch::Tensor f(torch::Tensor x);
""",
    cuda_sources="""// EXPECT:cuda
__global__ void k(float* x) { x[threadIdx.x] = 0; }
""",
)

load_inline(
    name="kw_list",
    cpp_sources=["// EXPECT:cpp", "// EXPECT:cpp"],
    cuda_sources=("// EXPECT:cuda",),
    sycl_sources=["// EXPECT:cpp"],
    functions=["// EXPECT:none"],
)

load_inline(
    name="kw_sycl_string",
    cpp_sources="// EXPECT:cpp",
    sycl_sources="""// EXPECT:cpp
sycl::queue q;
""",
    # Not a load_inline argument; ROCm builds hipify cuda_sources instead.
    hip_sources="// EXPECT:none",
)

# --- variable names (cuda) ---------------------------------------------------

cuda_src = "// EXPECT:cuda"
hip_src = "// EXPECT:cuda"
add_cuda = "// EXPECT:cuda"
add_cu = "// EXPECT:cuda"
add_hip = "// EXPECT:cuda"
add_kernel = r'''// EXPECT:cuda'''
kernel_code = "// EXPECT:cuda"
cuda_kernel = "// EXPECT:cuda"
self_like.cuda_source = "// EXPECT:cuda"
annotated_kernel: str = "// EXPECT:cuda"

# --- variable names (cpp) ----------------------------------------------------

cpp_src = "// EXPECT:cpp"
binding_cpp = "// EXPECT:cpp"
cpp_kernel = "// EXPECT:cpp"
self_like.cpp_source = "// EXPECT:cpp"

# --- variable names are case-insensitive ---------------------------------------

CUDA_SRC = "// EXPECT:cuda"
ADD_KERNEL = "// EXPECT:cuda"
Hip_Source = "// EXPECT:cuda"
CPP_SOURCE = "// EXPECT:cpp"
Binding_Cpp = "// EXPECT:cpp"
CPP_KERNEL = "// EXPECT:cpp"
GREETING = "hello // EXPECT:none"

# --- marker comments ---------------------------------------------------------

# cuda
templated = "// EXPECT:cuda"

# c++
wrapper = "// EXPECT:cpp"

# language=cuda
other = "// EXPECT:cuda"

# --- content based -----------------------------------------------------------

source = """// EXPECT:cuda
__global__ void scale(float* x, int n) {}
"""
idx_only = "int i = blockIdx.x * blockDim.x + threadIdx.x; // EXPECT:cuda"
decl = "torch::Tensor scale(torch::Tensor x); // EXPECT:cpp"
header = "#include <torch/extension.h> // EXPECT:cpp"

# --- f-strings: one injection spanning every literal segment -----------------

BLOCK = 256
templated_kernel = f"""// EXPECT:cuda
__global__ void k(float* x) {{ int i = {BLOCK}; }}
"""
back_to_back_cuda = f"// EXPECT:cuda {BLOCK}{BLOCK} int a; {BLOCK}{BLOCK} int b;"
load_inline(name="kw_fstring", cpp_sources=f"// EXPECT:cpp {BLOCK} int a; {BLOCK}{BLOCK} int b;")

# --- wrapped strings: dedent / cleandoc / strip and implicit concatenation ----
# Every rule above applies to these forms too. Concatenated pieces form one
# injection.

cuda_dedent = textwrap.dedent("""// EXPECT:cuda""")
cpp_dedent = dedent("// EXPECT:cpp")
cuda_cleandoc = inspect.cleandoc("// EXPECT:cuda")
cuda_strip = "// EXPECT:cuda".strip()
cpp_dedent_strip = textwrap.dedent("// EXPECT:cpp").lstrip()
cuda_concat = ("// EXPECT:cuda" "// EXPECT:cuda")
cpp_concat_bare = "// EXPECT:cpp" "// EXPECT:cpp"
dedented_source = textwrap.dedent("__global__ void k() {} // EXPECT:cuda")
stripped_decl = "torch::Tensor f(torch::Tensor x); // EXPECT:cpp".strip()

# cuda
marked_dedent = textwrap.dedent("// EXPECT:cuda")

# --- must not be injected ----------------------------------------------------

cuda_other_call = format_source("// EXPECT:none")
cuda_other_method = "// EXPECT:none".upper()
cuda_two_args = textwrap.dedent("// EXPECT:none", "// EXPECT:none")
wrapped_content = format_source("__global__ EXPECT:none")
# cuda
marked_call = format_source("// EXPECT:none")

greeting = "hello // EXPECT:none"
cuda_count = 3


def documented():
    """torch::Tensor in a docstring stays a docstring. EXPECT:none"""


print("__global__ in a plain call argument EXPECT:none")
