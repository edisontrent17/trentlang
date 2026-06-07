# Trentlang

Trentlang is a tiny C-like compiled language. Version 1 supports a single
`int main()` function, integer expressions, `print(expr);`, and `return expr;`.

The compiler is written in Rust. It emits LLVM IR and then asks `clang` to turn
that IR into a native executable.

## Setup

Install the required tools:

```bash
sudo apt-get update
sudo apt-get install -y cargo clang
```

Verify the toolchain:

```bash
cargo --version
rustc --version
clang --version
```

## Build

```bash
cargo build
```

## Compile A Program

```bash
cargo run -- examples/arithmetic.tl
./examples/arithmetic
```

The default command writes both:

- `examples/arithmetic.ll`
- `examples/arithmetic`

To only emit LLVM IR:

```bash
cargo run -- --emit-ir-only examples/arithmetic.tl
```

## Language V1

```c
int main() {
    print(1 + 2 * 3);
    return 0;
}
```

Supported:

- `int main() { ... }`
- integer literals
- `+`, `-`, `*`, `/`
- parentheses
- unary minus
- `print(expr);`
- `return expr;`
- `//` line comments

Not supported yet:

- variables
- functions besides `main`
- strings
- arrays
- conditionals
- loops
- pointers
