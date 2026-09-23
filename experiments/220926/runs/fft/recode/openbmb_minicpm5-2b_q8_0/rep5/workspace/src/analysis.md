# Source Project Analysis

## Files Found:
- Makefile
- src/fft.c
- src/fft.h
- tests/test.c

## Key Observations:
1. FFT library with Cooley-Tukey radix-2 FFT implementation
2. Uses GCC-specific builtins (__builtin_clz, _Generic) - only works on GCC/Clang
3. Test suite with known input/output for n=8
4. Static analysis with assert-based tests
5. Build system uses Makefile with profiling flags (-fprofile-arcs, -ftest-coverage)
6. Target size is determined by log2(N), supports powers of 2 only
7. Two APIs: fft() (out-of-place) and fft_inplace() (in-place)
8. Uses complex numbers with float components
9. GCC-specific optimization macros (likely/unlikely via __builtin_expect)
