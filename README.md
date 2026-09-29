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
deque TALK --cast F   record it played through, as an asciinema cast
deque TALK --share    stream it live on the network, for anyone who can't see
deque notes TALK      speaker notes and timer; drives the talk
deque check TALK      report problems
deque lsp             language server
```

The mouse is a laser pointer: a red dot with a glow that lights the text under it, a ring where you click, fading when it's still. The terminal's own arrow is hidden where it lets deque.

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
| ```` ```sh run ```` | code that runs on a step, its output under it |
| `![label](file)` | picture; side by side on one line |
| `// text` | speaker note |

Inline: `**bold**` `` `code` `` `{accent}…{/}` `${ENV}`

| option | values |
|---|---|
| `fx:` | `wipe` `iris` `slide` `zip` `drop` `fade` `scramble` `assemble` `glitch` `typewriter` `random` `none` |
| `lines:` | `type` `glide` `fade` `scramble` `count` `none` |
| `reveal:` | same as `lines:` |
| `then:` | `shine` `pulse` `shake` `rainbow` `sparkle` `confetti` |
| `tr:` | `dissolve` `sweep` `curtain` `morph` `life` `focus` `none` |
| `sky:` | `stars` `snow` `rain` `embers` `life` `boids` `fireflies` `none` |
| `glow:` | `on` `off`: the headline lights what's around it |
| `enter:` | command to run on enter |
| `cols:` | columns while `enter:` runs, with `--tv` |
| `draw:` | `WxH` canvas: `text` `center` `box` `arrow` `dotted` `clear` `step` |

Code in the same language on slides in a row morphs: what's in both swings to its new place, the rest fades. A slide's own `tr:` turns that off. `tr: morph` does it to the whole slide: every word, and the headline's blocks flocking into the new headline.

A `run` block runs in the talk's folder (`sh` `bash` `zsh` `fish` `py` `js` `rb`); a key stops it. Leave out the headline to give its output room.

`--cast talk.cast` is 100x30 unless `--size WxH` says; [agg](https://github.com/asciinema/agg) turns it into a GIF.

A `sky:` moves the whole time the slide is up, drawn a quarter of a cell at a time; the same sky on the next slide carries on. It fills the screen with the talk's `bg` color and fades out around text.

Headlines move a quarter of a cell at a time. In kitty and Ghostty, moving text goes as pictures in the terminal's own font, placed to the pixel; elsewhere it smears between cells. `DEQUE_FACE=path` picks the font, `DEQUE_SMOOTH=off` smears everywhere.

Options before the first `---` are defaults. Colors: `accent` `muted` `good` `bad` `warm` `link` `fg` `bg` (`#rrggbb`).

## Watching along

With `--share`, anyone on the same network can follow the talk live in a browser, animations and pointer included. Press `w` to show them how: a QR code and the link, over the slide. A slide can say `${DEQUE_URL}`.

The link carries a random token; without it there's nothing to see. Watchers get only what drawing needs (text, cursor moves, colors), never anything that would change their terminal, and can't send anything back. It's plain HTTP, so anyone on the network can read what's shown.

The page draws in your terminal's font: the file its config names (Ghostty, kitty) or `DEQUE_FACE`, sent to watchers' browsers. `--share-no-font` keeps it on your machine; check your font's license allows serving it.

`--share-curl` also offers `curl -sN …` to watch in a terminal (`${DEQUE_WATCH}`). Over plain HTTP, whoever's on the network between could write to that terminal, so use it on networks you trust. Pictures don't reach watchers.

## Images

kitty, iTerm2, or sixel, detected; half-blocks otherwise. Force with `DEQUE_IMAGES=kitty|iterm|sixel|blocks`.

## Neovim

```lua
{ "booka66/deque", lazy = false }
```

`:DequePreview` shows the slide under the cursor. `:DequePlay` replays it.
