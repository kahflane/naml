---
title: Editor Setup
description: Setting up VS Code and other editors for naml development
---

naml ships with a VS Code extension and a Language Server Protocol (LSP) server that works with any LSP-compatible editor.

## VS Code

### Install the Extension

Build and install from source:

```bash
# Build the LSP server
cargo build --release -p naml-lsp

# Build the VS Code extension
cd editors/vscode
npm install
npm run compile
npx vsce package
code --install-extension naml-0.1.0.vsix
```

### Configure the LSP Path

The extension needs to find the `naml-lsp` binary. Choose one of these methods:

**Option 1** — Add to PATH:
```bash
export PATH="$PATH:/path/to/naml/target/release"
```

**Option 2** — Copy to system bin:
```bash
sudo cp target/release/naml-lsp /usr/local/bin/
```

**Option 3** — Set in VS Code settings:
```json
{
    "naml.lsp.path": "/path/to/naml/target/release/naml-lsp"
}
```

### Features

The extension provides:

- **Syntax highlighting** for all naml constructs
- **Error diagnostics** — parse errors and type errors shown inline
- **Completions** — keywords, types, functions, methods, and module items (triggered by `.`, `:`, `{`, `,`)
- **Hover** — type signatures, function signatures, struct/enum definitions
- **Go to definition** — jump to function, type, and variable declarations
- **Find references** — locate all usages of a symbol
- **Document outline** — navigate functions, structs, and enums in the sidebar
- **Bracket matching** and auto-closing
- **Comment toggling** with `Cmd+/` / `Ctrl+/`

### Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `naml.lsp.path` | string | `""` | Custom path to `naml-lsp` binary |
| `naml.lsp.trace.server` | string | `"off"` | LSP trace level (`off`, `messages`, `verbose`) |

## Other Editors

The `naml-lsp` binary implements the Language Server Protocol and works with any LSP client.

### Neovim (nvim-lspconfig)

```lua
vim.api.nvim_create_autocmd('FileType', {
    pattern = 'naml',
    callback = function()
        vim.lsp.start({
            name = 'naml-lsp',
            cmd = { 'naml-lsp' },
            root_dir = vim.fs.dirname(vim.fs.find('naml.toml', { upward = true })[1]),
        })
    end,
})
```

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "naml"
scope = "source.naml"
file-types = ["nm"]
language-servers = ["naml-lsp"]

[language-server.naml-lsp]
command = "naml-lsp"
```

### Zed

Add to Zed settings:

```json
{
    "lsp": {
        "naml-lsp": {
            "binary": { "path": "naml-lsp" }
        }
    }
}
```
