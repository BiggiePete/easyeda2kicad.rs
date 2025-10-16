[![Rust](https://github.com/BiggiePete/easyeda2kicad.rs/actions/workflows/rust_test.yml/badge.svg)](https://github.com/BiggiePete/easyeda2kicad.rs/actions/workflows/rust_test.yml)


# easyeda2kicad.rs

**A Rust library for converting EasyEDA projects to KiCad formats**

## Overview

`easyeda2kicad.rs` is a Rust-based library and toolset designed to convert PCB design files from [EasyEDA](https://easyeda.com/) to [KiCad](https://kicad.org/) compatible formats. This project is inspired by and based on the original [easyeda2kicad.py](https://github.com/uPesy/easyeda2kicad.py) Python project, but is being transfered to Rust for nothing more than the sake of fun!

## Project Goals

- **Faithful Conversion:** Accurately translate EasyEDA schematic, PCB, symbol, and footprint data into KiCad's formats.
- **Rust-first Design:** Provide a robust, idiomatic Rust API for use in other Rust projects, as well as a CLI for end users.
- **Extensibility:** Make it easy to add support for new features, file formats, or workflows.
- **Performance:** Leverage Rust's speed and safety for fast, reliable conversions.

## Relationship to easyeda2kicad.py

This project is a reimplementation of the [easyeda2kicad.py](https://github.com/uPesy/easyeda2kicad.py) converter, which is a mature and widely used Python script for converting EasyEDA projects to KiCad. While the Python version is script-oriented, `easyeda2kicad.rs` is designed as a reusable Rust library, with the following differences:

- **Language:** Written in Rust, not Python.
- **Architecture:** Modular, library-first design for integration into other tools or GUIs.
- **Performance:** Faster execution and lower memory usage.
- **Safety:** Benefits from Rust's strict type system and memory safety.

## Features

- Parse EasyEDA JSON and project files
- Convert symbols, footprints, and 3D models to KiCad-compatible formats
- Write output files for use in KiCad projects
- CLI and library usage (in progress)

## Usage

**Library (Rust):**

```rust
use easyeda2kicad_rs::{importer, converter, file_writer};
// Example: Load an EasyEDA file, convert, and write KiCad output
```

**CLI:**

*CLI usage is planned for future releases.*

## Project Structure

- `src/` — Core Rust library code (importers, converters, writers)
- `examples/` — Example usage and test conversions

## Status

This project is under active development. Not all features from the Python version are implemented yet. Contributions and feedback are welcome!

## License

MIT License

---

**See also:**

- [easyeda2kicad.py (Python original)](https://github.com/uPesy/easyeda2kicad.py)
