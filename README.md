# Campfire 🏕️

Campfire (`cf`) is a private Rust CLI prototype for entering repo-defined toolbox shells.

This package is not ready for general release and is marked with `publish = false` to prevent accidental publication to crates.io.

## Development

Run the local checks before opening a pull request:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```
