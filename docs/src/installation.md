# Installation

## From crates.io

Install the latest published version with Cargo:

```bash
cargo install pctx
```

Verify the installation:

```bash
pctx --version
pctx --help
```

Update an existing installation:

```bash
cargo install pctx --force
```

Uninstall:

```bash
cargo uninstall pctx
```

## Build from source

```bash
git clone https://github.com/mc-marcocheng/pctx
cd pctx
cargo build --release
```

The compiled binary will be available at:

```bash
target/release/pctx
```

You can run it directly:

```bash
./target/release/pctx --help
```

Or install it locally from the checkout:

```bash
cargo install --path .
```

## Shell completions

Generate completions with:

```bash
pctx completions bash
pctx completions zsh
pctx completions fish
pctx completions powershell
pctx completions elvish
```

Example for Bash:

```bash
pctx completions bash > pctx.bash
source pctx.bash
```

Example for Zsh:

```bash
pctx completions zsh > _pctx
```

Then move `_pctx` into a directory listed in your `$fpath`.

## Build the documentation locally

If you have `mdbook` installed:

```bash
cd docs
mdbook serve
```

Then open the local URL printed by `mdbook`.
