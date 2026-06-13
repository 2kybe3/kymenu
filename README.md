**kymenu** — minimal Wayland overlay menu/launcher

A tiny, fast, dependency-light dmenu-style launcher for wlroots compositors.

### Features
- Pure Wayland (layer-shell)
- Built-in path launcher (`--path-launcher`)
- JSON input/output support
- Customizable colors, fonts, arrows, margins

### Quick Start

```bash
# Pipe items
echo "hi\nthere" | kymenu

# Path launcher
kymenu --path-launcher

# Custom styling
kymenu --background-color "#1e1e2e" --prompt "Run: " --font-size 18 --path-launcher
```

### Extras

See [kymenu-extras](https://git.kybe.xyz/2kybe3/kymenu-extras) for helpers and usage examples.
