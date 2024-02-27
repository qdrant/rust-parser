# Rust code parser

Extracts functions, enums, structs, into a JSON file.
Aims to provide a searchable representation of the code.

Usage:

```bash
cargo run -- $PATH_TO_RUST_PROJECT > structures.jsonl
```

Or Docker:

```bash
docker run --rm -v $PATH_TO_RUST_PROJECT:/source qdrant/rust-parser ./rust_parser /source > structures.jsonl
```
