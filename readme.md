# Tmux-fzy

A simple project manager for tmux.

# Screenshot

![](./Screenshot.png)

# Installation

## From source

```
cargo build --release
cargo install --path . --force
```

make sure you have `.cargo/bin` in your path

`export PATH="${PATH}:$HOME/.cargo/bin"`

# Usage

how to add dirs

```
tmux-fzy add --mindepth 1 --maxdepth 1 ~/Music
```

_dirs are stored in `XDG_CACHE_HOME/.tmux-fzy`_
