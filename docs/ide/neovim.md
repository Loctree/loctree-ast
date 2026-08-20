# Neovim Setup

Configure Neovim to use `loctree-lsp` for dead code detection and navigation.

## Prerequisites

- Neovim 0.8+
- [nvim-lspconfig](https://github.com/neovim/nvim-lspconfig)
- `loctree-lsp` installed. The fastest paths are `npm install -g @loctree/loctree` or `curl -fsSL https://loct.io/install.sh | bash`. Source builds are contributor-only fallback (`cargo build -p loctree-lsp`). Smoke-test with `loctree-lsp --version`.

The shipped config resolves an explicit `vim.g.loctree_lsp_path` first, then
prefers `~/.local/bin/loctree-lsp` over `PATH`. Run `:LoctreeRuntime` to see the
exact active executable, its full build identity, and resolution source. If an
older Cargo/Homebrew copy shadows the preferred install on `PATH`, Neovim warns
with both paths and identities while continuing with the preferred runtime.

## Configuration

Add to your Neovim config (`init.lua` or `lua/plugins/lsp.lua`):

```lua
-- Add loctree to lspconfig
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Define loctree LSP if not already defined
if not configs.loctree then
  configs.loctree = {
    default_config = {
      cmd = { 'loctree-lsp' },
      filetypes = {
        'typescript', 'typescriptreact',
        'javascript', 'javascriptreact',
        'rust', 'python', 'go', 'vue', 'svelte'
      },
      root_dir = lspconfig.util.root_pattern('.loctree', '.git'),
      settings = {},
    },
  }
end

-- Setup with your preferred options
lspconfig.loctree.setup({
  on_attach = function(client, bufnr)
    -- Your on_attach function
    -- Loctree provides: diagnostics, hover, definition, references
  end,
})
```

## Lazy.nvim Example

```lua
{
  'neovim/nvim-lspconfig',
  config = function()
    local lspconfig = require('lspconfig')
    local configs = require('lspconfig.configs')

    if not configs.loctree then
      configs.loctree = {
        default_config = {
          cmd = { 'loctree-lsp' },
          filetypes = { 'typescript', 'javascript', 'rust', 'python' },
          root_dir = lspconfig.util.root_pattern('.loctree', '.git'),
        },
      }
    end

    lspconfig.loctree.setup({})
  end,
}
```

## Features

### Diagnostics

Dead exports, cycles, and twins appear as LSP diagnostics:

```
W: Export 'unusedFunction' has 0 imports [loctree:dead-export]
W: Circular import: a.ts → b.ts → a.ts [loctree:cycle]
I: Symbol 'Config' also exported from 3 files [loctree:twin]
```

### Hover

`:lua vim.lsp.buf.hover()` or `K` shows:

```
Export: useAuth
─────────────────
12 imports across 8 files
Top consumers: App.tsx, Login.tsx, Dashboard.tsx
```

### Go to Definition

`gd` jumps to the original export location, resolving re-export chains.

### References

`gr` lists all files importing the symbol.

## Keybindings

Suggested mappings (add to your config):

```lua
vim.keymap.set('n', 'gd', vim.lsp.buf.definition, { desc = 'Go to definition' })
vim.keymap.set('n', 'gr', vim.lsp.buf.references, { desc = 'Find references' })
vim.keymap.set('n', 'K', vim.lsp.buf.hover, { desc = 'Hover info' })
vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, { desc = 'Code actions' })
vim.keymap.set('n', '<leader>lr', ':!loct<CR>', { desc = 'Refresh loctree' })
```

## Troubleshooting

### LSP not starting

```vim
:LspInfo
```

Check if loctree is listed and running.

### No diagnostics

Ensure `.loctree/snapshot.json` exists:

```bash
loct  # Generate snapshot
```

### Check LSP logs

```vim
:lua vim.cmd('edit ' .. vim.lsp.get_log_path())
```

---

*𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI*
