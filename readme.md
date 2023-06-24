# Tmux-fzy

A simple project manager for tmux that allows you to easily switch between sessions and open new sessions.

# Screenshot

![](./Screenshot.png)

# Installation

`cd` into the cloned repo

```
cargo build --release
cargo install --path .
```

make sure you have `.cargo/bin` in your path

`export PATH="${PATH}:$HOME/.cargo/bin"`

# Usage

this will add all the subdirs in the directory to the list
```
tmux-fzy add --mindepth 1 --maxdepth 1 ~/Music
```
