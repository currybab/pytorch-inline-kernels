// Open in Zed to check the CUDA language: qualifiers, builtins and <<< >>>.
#include <cuda_runtime.h>

__constant__ float kScale = 2.0f;

__device__ __forceinline__ float twice(float v) { return v * kScale; }

__global__ void __launch_bounds__(256) vector_add(const float* a, const float* b, float* c, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        c[i] = twice(a[i]) + b[i];
        atomicAdd(&c[0], 0.0f);
    }
}

int main() {
    const int n = 1 << 20;
    float *a, *b, *c;
    cudaMallocManaged(&a, n * sizeof(float));
    cudaMallocManaged(&b, n * sizeof(float));
    cudaMallocManaged(&c, n * sizeof(float));
    vector_add<<<(n + 255) / 256, 256>>>(a, b, c, n);
    cudaDeviceSynchronize();
    cudaFree(a);
    cudaFree(b);
    cudaFree(c);
    return 0;
}
