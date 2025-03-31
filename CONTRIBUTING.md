# Contributing to xui-rs

First off, thank you for considering contributing to `xui-rs`! Your help is appreciated.

## How Can I Contribute?

There are many ways to contribute, from writing code and documentation to reporting bugs and suggesting features.

### Reporting Bugs

* **Ensure the bug was not already reported** by searching on GitHub under [Issues](https://github.com/LineGM/xui-rs/issues).
* If you're unable to find an open issue addressing the problem, [open a new one](https://github.com/LineGM/xui-rs/issues/new). Be sure to include a **title and clear description**, as much relevant information as possible, and a **code sample** or an executable test case demonstrating the expected behavior that is not occurring.

### Suggesting Enhancements

* Open a new issue to discuss your enhancement idea. Explain why this enhancement would be useful and how it might be implemented.

### Pull Requests

1.  **Fork the repository** on GitHub.
2.  **Clone your fork** locally: `git clone https://github.com/LineGM/xui-rs.git`.
3.  **Create a new branch** for your changes: `git checkout -b feature/your-feature-name` or `git checkout -b fix/your-bug-fix-name`.
4.  **Make your changes.** Ensure you:
    * Follow the existing code style.
    * Add documentation (doc comments) for new public APIs.
    * Add tests for your changes: `cargo test`.
    * Format your code: `cargo fmt`.
    * Check for common issues: `cargo clippy`.
    * Ensure the code builds: `cargo build`.
5.  **Commit your changes** with clear and descriptive commit messages.
6.  **Push your branch** to your fork on GitHub: `git push origin feature/your-feature-name`.
7.  **Open a Pull Request (PR)** against the `main` branch of the original repository.
    * Provide a clear title and description for your PR.
    * Link to any relevant issues (e.g., "Closes #123").

## Development Setup

1.  Install Rust and Cargo: [https://www.rust-lang.org/tools/install.](https://www.rust-lang.org/tools/install)
2.  Clone the repository: `git clone https://github.com/LineGM/xui-rs.git`.
3.  Build the project: `cd xui-rs && cargo build`.
4.  Run tests: `cargo test`.
5.  Run the example (modify `src/main.rs` with your panel details if needed): `cargo run`.

## Code of Conduct

Please note that this project is released with a Contributor [**Code of Conduct**](CODE_OF_CONDUCT.md). By participating in this project you agree to abide by its terms.

Thank you for your contribution!
