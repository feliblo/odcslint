# Installation

odcslint is a single executable. Install it with whichever package manager you already use; the pip package contains the same binary, so you don't need Python to run it.

=== "pip / uv"

    ```bash
    pip install odcslint
    # or, as a standalone tool
    uv tool install odcslint
    # or, as a dev dependency of your project
    uv add --dev odcslint
    ```

=== "Homebrew"

    ```bash
    brew install feliblo/tap/odcslint
    ```

=== "Shell (macOS / Linux)"

    ```bash
    curl --proto '=https' --tlsv1.2 -LsSf https://github.com/feliblo/odcslint/releases/latest/download/odcslint-installer.sh | sh
    ```

=== "PowerShell (Windows)"

    ```powershell
    powershell -ExecutionPolicy Bypass -c "irm https://github.com/feliblo/odcslint/releases/latest/download/odcslint-installer.ps1 | iex"
    ```

=== "From source"

    ```bash
    cargo install --git https://github.com/feliblo/odcslint
    ```

Prebuilt binaries are available for Linux (x86_64 and aarch64), macOS (Intel and Apple Silicon) and Windows (x86_64). Building from source needs a Rust toolchain.

To check that it works:

```bash
odcslint --version
```
