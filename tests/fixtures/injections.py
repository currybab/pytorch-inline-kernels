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
    hip_sources=["// EXPECT:cuda"],
    functions=["// EXPECT:none"],
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

# --- must not be injected ----------------------------------------------------

greeting = "hello // EXPECT:none"
cuda_count = 3


def documented():
    """torch::Tensor in a docstring stays a docstring. EXPECT:none"""


print("__global__ in a plain call argument EXPECT:none")
