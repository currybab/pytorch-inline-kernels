# vim: set ft=load_inline:
"""Open this in Zed after installing the extension to check the highlighting.

The modeline above switches the file to "Python (load_inline)"; the status bar
should show that language. Each string below is highlighted as C++ or CUDA.
"""

import torch
from torch.utils.cpp_extension import load_inline

cpp_source = """
#include <torch/extension.h>

torch::Tensor scale(torch::Tensor x, double factor);
"""

cuda_source = """
#include <torch/extension.h>

__global__ void scale_kernel(const float* __restrict__ x, float* out, float factor, int n) {
    __shared__ float tile[256];
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        tile[threadIdx.x] = x[i] * factor;
        __syncthreads();
        out[i] = tile[threadIdx.x];
    }
}

torch::Tensor scale(torch::Tensor x, double factor) {
    auto out = torch::empty_like(x);
    int n = x.numel();
    int threads = 256;
    int blocks = (n + threads - 1) / threads;
    scale_kernel<<<blocks, threads>>>(x.data_ptr<float>(), out.data_ptr<float>(), factor, n);
    return out;
}
"""

# f-string kernels stay one C++ document across the {interpolations}.
BLOCK = 128
templated_kernel = f"""
__global__ void fill(float* out, int n) {{
    int i = blockIdx.x * {BLOCK} + threadIdx.x;
    if (i < n) out[i] = 1.0f;
}}
"""

module = load_inline(
    name="scale_ext",
    cpp_sources=[cpp_source],
    cuda_sources=[cuda_source],
    functions=["scale"],
    verbose=True,
)

if __name__ == "__main__":
    x = torch.arange(8, dtype=torch.float32, device="cuda")
    print(module.scale(x, 2.0))
