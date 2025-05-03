# swaytreesave

> CLI to save and load your sway tree

## Features

- Save and load your sway tree
- Exec customisation
- Timeout customisation per item
- Retry customisation per item
- Save and load multiple tree giving a name
- Load specific workspace of specific tree

## Installation

### With cargo

```bash
git clone git@github.com:fabienjuif/swaytreesave.git
cargo install --path ./swaytreesave
```

### On Void Linux

You can find a custom [template here.](https://github.com/fabienjuif/void-packages/pull/4)

## Usage

**swaytreesave --help**

```bash
Usage: swaytreesave [OPTIONS] <COMMAND>

Commands:
  save  Save your current sway tree
  load  Load a sway tree
  help  Print this message or the help of the given subcommand(s)

Options:
      --name <NAME>  Name of your tree
      --dry-run      Dry run
      --no-kill      No kill
  -h, --help         Print help
  -V, --version      Print version
```

**swawytreesave load --help**

```bash
Load a sway tree

Usage: swaytreesave load [OPTIONS]

Options:
      --workspace <WORKSPACE>  Specify the workspace to load. Other workspaces app will not be killed, and only this workspace apps will be loaded from config file
  -h, --help                   Print help
```

### Example

Saves the current tree to `$HOME/.config/swaytreesave/default.yaml`:

```bash
swawytreesave save
```

Loads the default tree back:

```bash
swawytreesave load
```

### Sway config example

```bash
# trees loader
# needs https://github.com/fabienjuif/swaytreesave
set $treeload_mode 'load tree (d|p:default, w:work)'
mode $treeload_mode {
    bindsym d exec swaymsg 'mode "default"' && swaytreesave load
    bindsym p exec swaymsg 'mode "default"' && swaytreesave load
    bindsym w exec swaymsg 'mode "default"' && swaytreesave --name work load

    bindsym Return mode "default"
    bindsym Escape mode "default"
}
bindsym $mod+Shift+t mode $treeload_mode
```
