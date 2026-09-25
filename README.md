# deque

Slides in the terminal, from a text file.

## Install

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/booka66/deque/releases/latest/download/deque-installer.sh | sh
```

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/booka66/deque/releases/latest/download/deque-installer.ps1 | iex"
```

Or `cargo install --git https://github.com/booka66/deque`.

## Use

```
deque TALK [N]        present, from slide N
deque TALK --print    print every slide
deque TALK --tv       fullscreen in a new Ghostty window (macOS)
deque notes TALK      speaker notes and timer; drives the talk
deque check TALK      report problems
deque lsp             language server
```

Keys: `→` `space` `n` next · `←` `b` back · `12⏎` go to 12 · `o` overview · `r` replay · `q` quit

Saving the talk reloads it. `deque demo/talk.deque` shows every feature.

## Format

```
#!/usr/bin/env deque
fx: wipe

---
# HELLO
**subtitle**

---
fx: scramble

## label
# HEADLINE
a line
> a step

---
enter: htop

# DEMO

---
![before](a.png) ![after](b.png)
caption
```

| line | |
|---|---|
| `## text` | label |
| `# TEXT` | headline, block letters |
| `text` | centered line |
| `> text` | step |
| ```` ``` ```` | column-aligned block; ```` ```lang ```` highlights code |
| `![label](file)` | picture; side by side on one line |
| `// text` | speaker note |

Inline: `**bold**` `` `code` `` `{accent}…{/}` `${ENV}`

| option | values |
|---|---|
| `fx:` | `wipe` `iris` `slide` `zip` `drop` `fade` `scramble` `assemble` `glitch` `typewriter` `random` `none` |
| `lines:` | `type` `glide` `fade` `scramble` `count` `none` |
| `reveal:` | same as `lines:` |
| `then:` | `shine` `pulse` `shake` `rainbow` `sparkle` `confetti` |
| `tr:` | `dissolve` `sweep` `curtain` `none` |
| `enter:` | command to run on enter |
| `cols:` | columns while `enter:` runs, with `--tv` |
| `draw:` | `WxH` canvas: `text` `center` `box` `arrow` `dotted` `clear` `step` |

Options before the first `---` are defaults. Colors: `accent` `muted` `good` `bad` `warm` `link` `fg` `bg` (`#rrggbb`).

## Images

kitty, iTerm2, or sixel, detected; half-blocks otherwise. Force with `DEQUE_IMAGES=kitty|iterm|sixel|blocks`.

## Neovim

```lua
{ "booka66/deque", lazy = false }
```

`:DequePreview` shows the slide under the cursor. `:DequePlay` replays it.
