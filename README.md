# Parsley: Rust Edition

Parsley is my attempt at writing a library for writing lexers and parsers with ease. This implementation is written in Rust, however, I may create versions for Go, or provide Go bindings for this package.

For now, this README will be rather, devoid of content.

## TODO

1. Write proper unit tests.
2. Clean the source code up.
   1. Make the API less, messy.
   2. Rework spans more cleanly.
3. Write proper documentation.
4. Make the Lexer type an iterator when lexing, allowing for a stream of tokens from a stream of bytes.
