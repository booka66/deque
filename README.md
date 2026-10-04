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
deque TALK --share    stream it live, a link for anyone who can't see; they can react and vote
deque TALK --rehearse a practice run: each slide's time written into the talk
deque TALK --loop     it goes on by itself, round and round: for a screen left on
deque notes TALK      speaker notes and timer; drives the talk
deque check TALK      report problems, and slides cut off at 80x24 (--size WxH)
deque lsp             language server
```

TALK can be a folder with a `talk.deque` in it, or left out when the folder you're in has one.

The mouse is a laser pointer: a red dot with a glow that lights the text under it, a ring where you click, fading when it's still. The terminal's own arrow is hidden where it lets deque.

Keys: `→` `space` `n` next · `←` `b` back · `12⏎` go to 12 · `'` back to where you jumped from · `o` overview (`/` finds a slide by its words or notes) · `r` replay · `B` or `.` blank to the sky, any key back · `v` an ant colony's other view, `i` how it's doing · `?` every key · `q` `q` quit (one `q` could be a slip; `ctrl-c` quits at once)

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
| ```` ```ts src/a.ts#fee@HEAD~2 ```` | the same, as it was at a revision in git |
| ```` ```ts focus: 2\|4-5 ```` | on each step, those lines lit, the rest dim |
| ```` ```chart ```` | lines of `label value`: bars that grow in; a value can be a `${ENV}` |
| ```` ```graph ```` | lines of `a -> b -> c`: boxes and arrows, laid out for you |
| ```` ```poll ```` | a choice a line: with `--share`, watchers vote and the bars grow as they do; a `* choice` is the answer, lit on the next step |
| `\| a \| b \|` | a table; a `\|---\|` row under the first makes it a header |
| `![label](file)` | picture; side by side on one line |
| `// text` | speaker note |

Inline: `**bold**` `` `code` `` `{accent}…{/}` `${ENV}` `${sh: command}`

| option | values |
|---|---|
| `fx:` | `wipe` `iris` `slide` `zip` `drop` `fade` `scramble` `assemble` `glitch` `typewriter` `random` `none` |
| `lines:` | `type` `glide` `fade` `scramble` `count` `none` |
| `reveal:` | same as `lines:` |
| `then:` | `shine` `pulse` `shake` `rainbow` `sparkle` `confetti` |
| `tr:` | `dissolve` `sweep` `curtain` `morph` `life` `sand` `focus` `none` |
| `sky:` | `stars` `snow` `rain` `embers` `life` `boids` `fireflies` `sand` `koi` `ants` `none`; a number after it is how many |
| `glow:` | `on` `off`: the headline lights what's around it |
| `enter:` | command to run on enter |
| `play:` | a recording (`.cast`) to play on enter instead |
| `cols:` | columns while `enter:` runs, with `--tv` |
| `keys:` | `on` `off`: while `enter:` runs, the keys you press on a row under it |
| `draw:` | `WxH` canvas: `text` `center` `box` `arrow` `dotted` `clear` `step` |
| `time:` | how long it's meant to be up (`90s`, `2m`); `deque notes` shows if you're ahead or behind, and `--loop` keeps it up that long |

Code in the same language on slides in a row morphs: what's in both swings to its new place, the rest fades. A slide's own `tr:` turns that off. `tr: morph` does it to the whole slide: every word, and the headline's blocks flocking into the new headline.

`play: demo.cast` plays a demo recorded beforehand (`asciinema rec demo.cast`) on enter, so it can't go wrong on the day: as it was recorded, long pauses cut short. It stops at the recording's markers (its `[time, "m", ""]` lines) for you to talk over; space pauses and goes on, `→` skips to the next marker, `q` stops. With an `enter:` too, enter cuts from the recording to the real thing, for questions. `deque check` warns when a recording is bigger than the screen.

`keys: on` is for a live demo of something driven by keys: the command gets the screen less its last row, and that row shows the last keys pressed, the newest lit, one pressed again as `j ×3`. They go after a couple of seconds. Watchers see them too. At the top of the talk, it's for every `enter:`. Don't type a password with it on.

A poll can have an answer: start that choice with `* `. Watchers aren't told which; the step after the votes dims the others and lights it. It works without `--share` too, for a show of hands.

A `run` block runs in the talk's folder (`sh` `bash` `zsh` `fish` `py` `js` `rb`); a key stops it. Leave out the headline to give its output room.

`--cast talk.cast` is 100x30 unless `--size WxH` says; [agg](https://github.com/asciinema/agg) turns it into a GIF.

A `sky:` moves the whole time the slide is up, drawn a quarter of a cell at a time; the same sky on the next slide carries on. It fills the screen with the talk's `bg` color and fades out around text.

Headlines move a quarter of a cell at a time. In kitty and Ghostty, moving text goes as pictures in the terminal's own font, placed to the pixel; elsewhere it smears between cells. `DEQUE_FACE=path` picks the font, `DEQUE_SMOOTH=off` smears everywhere.

Code from a file is read again when the file changes, and `deque check` says when the lines or the name aren't there.

A ```` ```graph ```` is a diagram without placing anything: each line names boxes with arrows between them, `->`, or `..>` for a dotted one, and `: text` after the last labels that arrow. ```` ```graph steps ```` starts with the first box alone and brings each arrow, with the box it reaches, on a key of its own. deque puts them in columns, left to right, each box one past the furthest box that points at it, and routes the arrows round the boxes, joined where they branch or meet (`├` `┬` `┼`); one that goes back, closing a loop, goes round outside everything. ```` ```graph down ```` lays it out top to bottom instead, for a narrow screen or a tall graph. A `> ` line comes in on a step, its new boxes traced in and its arrows drawn. Lines after the block are centered under it. The slide has a label, but no headline.

````
---
## how a request goes
```graph
browser -> api: POST /order
api -> db
api -> cache: read
> api ..> queue -> worker -> db
```
````

`@REV` after it takes it as it was in git: a commit, branch or tag. On slides in a row, `a.ts#fee@main` then `a.ts#fee` morphs the old function into the new one, a change walked through a slide at a time.

`--rehearse` is a practice run: present as you will on the day, and when you quit, the time you spent on each slide goes into the talk as its `time:` (to the nearest 5 seconds), a slide's own `time:` changed if it had one. `deque notes` then shows, on the day, whether you're ahead of your practice or behind it. Slides you didn't get to are left as they were.

The sky knows what's on the slide. Snow lies a while on the text it lands on; boids perch on a line, and scatter from the pointer; sand heaps on text and on the floor, and runs out when it's deep. `sky: koi` is a pond: koi swimming under lily pads, light crossing the water. Click to feed them: food scatters on the water where you clicked, and each fish makes for the nearest bit and eats it.

`sky: ants` is an ant colony, kept. After it go any of a view, a species and what the nest is in, and how many workers at most: `sky: ants`, `sky: ants ground fire sand 80`, `sky: ants gel black`.

| | |
|---|---|
| `farm` | the colony from the side (the default) |
| `ground` | the same colony from above |
| `leafcutter` | rust-brown, with majors, the big ones; they cut leaves (the default) |
| `black` | garden ants; they milk aphids, and take what's fallen |
| `fire` | red, quick, and half as many again |
| `soil` `sand` `gel` | what the nest is in: brown earth (the default), pale sand, or the blue gel of a see-through farm |
| `founding` `grown` | a queen alone with her first eggs (the default), or a colony well under way |

It's one colony, seen two ways. From the side it's the nest; from above, the same ants, those that are out. `v` goes from one view to the other, and two panes can show both at once: run the same talk in each and press `v` in one. An ant that goes down the hole in one is coming down the shaft in the other.

It begins with a queen alone at the bottom of the shaft she dug, with her first eggs (`grown` skips to a colony well under way). The first workers come small, and soon. From then it's theirs: they dig the galleries and chambers a grain at a time, each grain carried up and dropped round the door, where a mound grows; they bring in food; she lays while there's food to lay on.

**What an ant does goes by what it is and how old.** The smallest (minims) and the young stay in, with the brood and the queen. The middle-aged dig, and carry the dead to the midden. The old go out for food. The biggest (majors) keep watch by the door, and fight. When too few are digging or foraging, whoever's free does that.

**Brood** is carried up to the nursery by day, where it's warm, and down to the queen's chamber at night: eggs, then larvae, then pupae, then pale new ants. **Underground, two that meet stop head to head a moment**, one feeding the other. **A forager that finds plenty goes straight back for more, and leads another to it**, nose to tail; after that the scent does it, and a trail forms, round the words on the slide, and fades when the food's gone.

**Each kind lives its own way.** Leafcutters cut leaf from the plants and carry it home overhead, a minim sometimes riding on the piece; in the nest it's worked into the fungus garden, which is what they eat, and a garden not fed dwindles. Garden ants (`black`) milk the aphids on the plants, and take what's fallen. Fire ants take what's fallen, and all of them fight.

**It has days and nights and seasons**, by a clock of its own: a day is eight minutes, a year sixteen days. By night the foragers stay in, under stars. In late autumn the leaves fall; in winter there's snow, nobody goes out, the queen stops laying, and they cluster round her. **Rain** comes now and then: the door's stopped up against it, the top of the shaft takes in water, the trails wash away, and after it they start again.

**Once a year, in summer, a colony that's grown and fed raises the winged.** They wait in the nest till they're all there and the day's bright; then they come out, and fly.

**It isn't alone.** Twigs fall and lie in the way till they've rotted. A spider comes hunting: it eats a few of those that are out, and goes, unless enough of those that fight bring it down, and then it's food. Across the way is another colony, whose foragers take the same food; where one of theirs meets one of ours they square up, and sometimes one dies.

**For its keeper:** `i` shows how it's doing on the top row: the day, the season and the hour, how many workers of each size, the brood by stage, the food or the fungus, and who's about. The colony is kept on disk (under `~/.deque/colonies`), written every ten seconds, so it's there next time, older; `--fresh` begins it anew. It waits while deque isn't running, or is on a slide without it. A window resized keeps it, everything where it was, in proportion. Text on the slide is stuck on the glass: the nest goes on behind it.

A click drops food, in either. `deque demo/ants.deque --loop` shows every kind, and goes round by itself; `deque demo/antfarm.deque` is a farm and nothing else, for a screen left on.

A number after a sky is how many: `sky: rain 200`, `sky: koi 3`; for boids, birds and then hawks: `sky: boids 12 3`. With a command for the number, the sky shows something: a bird for every open issue, rain as heavy as the failures.

`${sh: command}` is what a command prints, anywhere `${ENV}` can go: a line, a headline, a chart's number, a sky's. It's run by `sh` (`cmd` on Windows) in the talk's folder when the talk is read, and again each minute while the talk's up; what changed on the slide swings to its new place, a headline's blocks into the new headline. `# ${sh: date +%H:%M}` is a clock. To show a `${` on a slide as it's written, double the dollar: `$${`. A talk with one runs commands when it's opened, not only on a key: read a talk you didn't write before you present it. The editor's preview doesn't run them, and shows `0`.

`--loop` is for a screen left on. Each slide stays up for its `time:` (10 seconds without one), a step at a time, then the next comes; after the last, the first. A `run` block runs again each time round: what it printed last time stays up while it runs, then turns into what it printed now, lines in both moving to their new places. With one slide, the slide stays as it is and only that happens. The keys still work.

```
sky: boids ${sh: gh issue list --assignee @me | wc -l} 2
time: 60s

## mine
# ${sh: date +%H:%M}
```sh run
gh issue list --assignee @me
```
```

`--calm` (or `calm: on`) keeps still what needn't move: no skies, glow, flourishes, transitions or morphs; things fade in.

Options before the first `---` are defaults. Colors: `accent` `muted` `good` `bad` `warm` `link` `fg` `bg` (`#rrggbb`).

## Watching along

With `--share`, anyone you give the link to can follow the talk live in a browser, animations and pointer included. Press `w` to show them how: a QR code and the link, over the slide. A slide can say `${DEQUE_URL}`.

The link works from anywhere, with no browser warning: deque opens a Cloudflare quick tunnel (`brew install cloudflared`; no account) and waits till the link can be found before the talk starts, a few seconds. It closes when deque does. The link's secret, a random token in it, but anyone it's passed to can watch, from anywhere, and the talk goes through Cloudflare on its way. A watcher on the same network as you gets the talk straight from your machine instead: the page, once it's loaded, connects to your laptop directly (WebRTC), in a second or so, as quick as the network. It's encrypted, and the page checks it's talking to your deque by a fingerprint it got over the trusted link. Watchers elsewhere, or on a network that keeps devices apart (some guest Wi-Fi does), stay on the tunnel, a few tens of milliseconds behind. Your phone remote does the same the other way: on your network, where you point goes straight to your laptop.

`--share-local` keeps it to this network, never through Cloudflare: for a venue with no internet, or a talk that mustn't leave the room. `--share` falls back to it, and says so on `w`'s card, when it can't open a tunnel (no `cloudflared`, or no internet).

The page has a row of emoji under the talk: a tap sends one floating up the right of the screen, for everyone (not with `--calm`). On a slide with a ```` ```poll ````, it has the choices too: a tap votes, a second tap on another changes the vote, and the bars on the screen move as the votes come in. The slide's headline is the question. Votes are kept while deque runs, so going back to a poll shows how it stood.

With `--share-local`, the link is HTTPS with a certificate deque makes for the run: browsers warn once (it's signed by nobody), then the stream is encrypted. Either way it carries a random token; without it there's nothing to see. Watchers get only what drawing needs (text, cursor moves, colors), never anything that would change their terminal. What they send back is only a number: which emoji, or which choice. No words of theirs reach the screen, and only so many reactions show at once.

The page draws in your terminal's font: the file its config names (Ghostty, kitty) or `DEQUE_FACE`, sent to watchers' browsers. `--share-no-font` keeps it on your machine; check your font's license allows serving it.

With `--share`, `P` (capital) shows a QR code for your phone: a remote with next and back, your notes, a timer, and a big red **hold to point** button. Hold it and aim the phone: the pointer shows on the screen, starting in the middle, and follows the phone as it turns; let go and it's gone, like a laser pointer. There's nothing to set up and nothing drifts: it goes by how the phone turns, not where it points, however you hold it or roll your wrist; if it's off, turn past the edge and it catches up, as a mouse does. A quick press is a ring; **speed** sets how far a turn goes. On an iPhone, the first tap asks to use the phone's motion. The page doesn't zoom, stays upright however the phone turns (locked to portrait where the browser lets it), and keeps the phone awake. The remote only works over HTTPS. It has its own link, apart from the watching one, and `deque notes` shows it too. Show it before the room's watching: anyone who scans it can drive the talk.

`--share-curl` also offers `curl -skN …` to watch in a terminal, on your network (`${DEQUE_WATCH}`). `-k` takes the unsigned certificate, so whoever's on the network between could still write to that terminal: use it on networks you trust. Pictures don't reach watchers.

## Images

kitty, iTerm2, or sixel, detected; half-blocks otherwise. Force with `DEQUE_IMAGES=kitty|iterm|sixel|blocks`.

## Neovim

```lua
{ "booka66/deque", lazy = false }
```

`:DequePreview` shows the slide under the cursor. `:DequePlay` replays it.
