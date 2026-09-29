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
deque new NAME        write NAME.deque, a talk to start from
deque TALK [N]        present, from slide N
deque TALK --print    print every slide
deque TALK --tv       fullscreen in a new Ghostty window (macOS)
deque TALK --cast F   record it played through, as an asciinema cast
deque TALK --html F   the same, as one page to post after the talk
deque TALK --share    stream it live on the network, for anyone who can't see
deque notes TALK      speaker notes and timer; drives the talk
deque check TALK      report problems, and slides cut off at 80x24 (--size WxH)
deque lsp             language server
```

TALK can be a folder with a `talk.deque` in it, or left out when the folder you're in has one.

The mouse is a laser pointer: a red dot with a glow that lights the text under it, a ring where you click, fading when it's still. The terminal's own arrow is hidden where it lets deque.

Keys: `→` `space` `n` next · `←` `b` back · `12⏎` go to 12 · `'` back to where you jumped from · `o` overview (`/` finds a slide by its words or notes) · `r` replay · `B` or `.` blank to the sky, any key back · `?` every key · `q` `q` quit (one `q` could be a slip; `ctrl-c` quits at once)

Saving the talk reloads it. Starting again, `'` goes back to the slide you left off on. When the window's too small for a slide, deque says so over it rather than cutting it off. `deque demo/talk.deque` shows every feature.

`--html` writes a page that plays the talk with `→` and `←`, a step at a time, animations included, scaled to the window. It loads xterm.js from jsDelivr (checked against its hash) and names your terminal's font rather than including it, so viewers who have it see it and the rest get a monospace.

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
| ```` ```ts src/a.ts#fee ```` | code from a file: `path`, `path:10-24`, or `path#name` (that function) |
| ```` ```ts focus: 2\|4-5 ```` | on each step, those lines lit, the rest dim |
| ```` ```chart ```` | lines of `label value`: bars that grow in |
| `\| a \| b \|` | a table; a `\|---\|` row under the first makes it a header |
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
| `time:` | how long it's meant to be up (`90s`, `2m`); `deque notes` shows if you're ahead or behind |

Code in the same language on slides in a row morphs: what's in both swings to its new place, the rest fades. A slide's own `tr:` turns that off. `tr: morph` does it to the whole slide: every word, and the headline's blocks flocking into the new headline.

A `run` block runs in the talk's folder (`sh` `bash` `zsh` `fish` `py` `js` `rb`); a key stops it. Leave out the headline to give its output room.

`--cast talk.cast` is 100x30 unless `--size WxH` says; [agg](https://github.com/asciinema/agg) turns it into a GIF.

A `sky:` moves the whole time the slide is up, drawn a quarter of a cell at a time; the same sky on the next slide carries on. It fills the screen with the talk's `bg` color and fades out around text.

Headlines move a quarter of a cell at a time. In kitty and Ghostty, moving text goes as pictures in the terminal's own font, placed to the pixel; elsewhere it smears between cells. `DEQUE_FACE=path` picks the font, `DEQUE_SMOOTH=off` smears everywhere.

Code from a file is read again when the file changes, and `deque check` says when the lines or the name aren't there.

`--calm` (or `calm: on`) keeps still what needn't move: no skies, glow, flourishes, transitions or morphs; things fade in.

Options before the first `---` are defaults. Colors: `accent` `muted` `good` `bad` `warm` `link` `fg` `bg` (`#rrggbb`).

## Watching along

With `--share`, anyone on the same network can follow the talk live in a browser, animations and pointer included. Press `w` to show them how: a QR code and the link, over the slide. A slide can say `${DEQUE_URL}`.

The link is HTTPS, with a certificate deque makes for the run: browsers warn once (it's signed by nobody), then the stream is encrypted. It carries a random token; without it there's nothing to see. Watchers get only what drawing needs (text, cursor moves, colors), never anything that would change their terminal, and can't send anything back.

The page draws in your terminal's font: the file its config names (Ghostty, kitty) or `DEQUE_FACE`, sent to watchers' browsers. `--share-no-font` keeps it on your machine; check your font's license allows serving it.

With `--share`, `P` (capital) shows a QR code for your phone: a remote with next and back, your notes, a timer, and a big red **hold to point** button. Hold it and aim the phone: the pointer shows on the screen, starting in the middle, and follows the phone as it turns; let go and it's gone, like a laser pointer. There's nothing to set up and nothing drifts: it goes by how the phone turns, not where it points, however you hold it or roll your wrist; if it's off, turn past the edge and it catches up, as a mouse does. A quick press is a ring; **speed** sets how far a turn goes. On an iPhone, the first tap asks to use the phone's motion. The page doesn't zoom, stays upright however the phone turns (locked to portrait where the browser lets it), and keeps the phone awake. The remote only works over HTTPS. It has its own link, apart from the watching one, and `deque notes` shows it too. Show it before the room's watching: anyone who scans it can drive the talk.

`--share-curl` also offers `curl -skN …` to watch in a terminal (`${DEQUE_WATCH}`). `-k` takes the unsigned certificate, so whoever's on the network between could still write to that terminal: use it on networks you trust. Pictures don't reach watchers.

## Images

kitty, iTerm2, or sixel, detected; half-blocks otherwise. Force with `DEQUE_IMAGES=kitty|iterm|sixel|blocks`.

## Neovim

```lua
{ "booka66/deque", lazy = false }
```

`:DequePreview` shows the slide under the cursor. `:DequePlay` replays it.
