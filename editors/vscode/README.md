# naml VS Code Extension

Language support for the naml programming language.

## Features

- Syntax highlighting for all naml constructs
- Error diagnostics (parse errors and type errors)
- Bracket matching and auto-closing
- Comment toggling (`Cmd+/` or `Ctrl+/`)
- Basic keyword completion

## Prerequisites

- [Node.js](https://nodejs.org/) (v18 or later)
- [Rust](https://rustup.rs/) (for building the LSP server)
- VS Code 1.85.0 or later

---

## Production Installation

### Step 1: Build the LSP Server

```bash
# From the naml project root
cargo build --release -p naml-lsp
```

### Step 2: Install the LSP Binary

Choose one option:

**Option A: Add to PATH**
```bash
# Add to your shell profile (~/.zshrc, ~/.bashrc, or ~/.bash_profile)
export PATH="$PATH:/path/to/naml/target/release"

# Then reload your shell
source ~/.zshrc  # or ~/.bashrc
```

**Option B: Copy to system bin**
```bash
sudo cp target/release/naml-lsp /usr/local/bin/
```

**Option C: Configure in VS Code settings**
```json
{
  "naml.lsp.path": "/path/to/naml/target/release/naml-lsp"
}
```

### Step 3: Build the Extension

```bash
cd editors/vscode
npm install
npm run compile
```

### Step 4: Package and Install

```bash
# Install vsce if not already installed
npm install -g @vscode/vsce

# Package the extension
vsce package

# Install the extension
code --install-extension naml-0.1.0.vsix
```

### Step 5: Reload VS Code

Press `Cmd+Shift+P` (Mac) or `Ctrl+Shift+P` (Windows/Linux), then type "Developer: Reload Window"

---

## Development Installation

### Step 1: Build the LSP Server (Debug)

```bash
# From the naml project root
cargo build -p naml-lsp
```

### Step 2: Install Dependencies

```bash
cd editors/vscode
npm install
```

### Step 3: Compile TypeScript

```bash
npm run compile

# Or watch for changes
npm run watch
```

### Step 4: Option A - Launch Extension Development Host

1. Open `editors/vscode` folder in VS Code
2. Press `F5` to launch Extension Development Host
3. A new VS Code window opens with the extension loaded
4. Open any `.naml` file to test

### Step 4: Option B - Symlink to Extensions Folder

```bash
# Create symlink (run from editors/vscode directory)
ln -s "$(pwd)" ~/.vscode/extensions/naml

# Reload VS Code
# Press Cmd+Shift+P -> "Developer: Reload Window"
```

To remove the symlink:
```bash
rm ~/.vscode/extensions/naml
```

### Step 5: Make Debug LSP Available

For development, point to the debug build:

```bash
# Add to PATH temporarily
export PATH="$PATH:/path/to/naml/target/debug"
```

Or configure in VS Code settings (`settings.json`):
```json
{
  "naml.lsp.path": "/path/to/naml/target/debug/naml-lsp"
}
```

---

## Troubleshooting

### Extension not activating

1. Check that `.naml` files are recognized:
   - Open a `.naml` file
   - Look at the bottom-right of VS Code status bar
   - It should show "naml" as the language

2. Check Output panel for errors:
   - `View` -> `Output`
   - Select "naml Language Server" from dropdown

### LSP not connecting

1. Verify the LSP binary exists and is executable:
   ```bash
   which naml-lsp
   # or
   ls -la /path/to/naml-lsp
   ```

2. Test the LSP manually:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | naml-lsp
   ```

3. Check VS Code settings for custom path:
   - `Cmd+,` (Settings)
   - Search for "naml.lsp.path"

### No syntax highlighting

1. Ensure the extension is installed:
   - `Cmd+Shift+X` (Extensions panel)
   - Search for "naml"

2. Check file association:
   - Open a `.naml` file
   - Click language indicator in status bar
   - Select "naml" if not already selected

### No error diagnostics

1. The LSP must be running for diagnostics
2. Check "naml Language Server" in Output panel
3. Syntax highlighting works without LSP, but diagnostics require it

---

## Configuration Options

Add to your VS Code `settings.json`:

```json
{
  // Path to naml-lsp binary (optional if in PATH)
  "naml.lsp.path": "/path/to/naml-lsp",

  // LSP trace level for debugging
  "naml.lsp.trace.server": "off"  // "off" | "messages" | "verbose"
}
```

---

## Project Structure

```
editors/vscode/
├── package.json                 # Extension manifest
├── tsconfig.json                # TypeScript config
├── language-configuration.json  # Brackets, comments, etc.
├── syntaxes/
│   └── naml.tmLanguage.json    # Syntax highlighting grammar
├── src/
│   └── extension.ts            # Extension entry point (LSP client)
└── out/                        # Compiled JavaScript (generated)
```

---

## Uninstalling

### Production Install
```bash
code --uninstall-extension naml-lang.naml
```

### Development Symlink
```bash
rm ~/.vscode/extensions/naml
```

---

## Building from Source

```bash
# Clone the naml repository
git clone https://github.com/kahflane/naml.git
cd naml

# Build everything
cargo build --release -p naml-lsp
cd editors/vscode
npm install
npm run compile
vsce package

# Install
code --install-extension naml-0.1.0.vsix
```
