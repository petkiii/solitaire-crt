# Solitaire CRT

Classic Klondike solitaire in Rust + Bevy, heavily inspired by Balatro.

## Run

```sh
cargo run -r
```

Running the compiled binary directly (not via cargo) needs the asset root:

```sh
BEVY_ASSET_ROOT=. ./target/release/solitaire-crt
```

## Features

- Full Klondike (draw-1, unlimited recycles) with standard scoring and a timer
- Undo, hints, and one-key auto-complete once every card is face-up
- Arcade card animations
- CRT post-processing: curvature, scanlines, color fringing, grain, vignette, bloom
- Animated shader background

## Controls

- **Drag** cards between piles; sequences drag together
- **Double-click** a card to send it somewhere useful (foundation first)
- **Click stock** to draw; click the empty stock slot to recycle the waste
- **Esc** pause menu (resume/new game/settings/quit), **Enter** starts a game
  from the title screen
- Keys in game: **D** draw/recycle, **U** undo, **H** hint, **N** new game,
  **Space/A** auto-complete (when available)

## Rules & scoring

Draw-1, unlimited recycles. Standard scoring: waste→tableau +5, →foundation
+10, tableau→foundation +10, card flip +5, foundation→tableau −15, recycle
−100, floored at 0.

## Structure

- `src/engine/` — headless Klondike engine: rules, moves, undo, hints
- `src/game.rs` — session state (scoring, timer), board↔engine sync, auto-complete, win cascade
- `src/deal.rs` — deal source (random; difficulty API planned)
- `src/animation.rs` — spring movement, tilt, pulses, card flips
- `src/input.rs` — drag & drop, clicks, keyboard shortcuts
- `src/layout.rs` — board geometry in virtual units
- `src/cards.rs` — card atlas indexing and face/back sprites
- `src/menu.rs`, `src/ui.rs`, `src/theme.rs` — title/pause/settings overlays, HUD, shared styling
- `src/background.rs` + `assets/shaders/background.wgsl` — animated backdrop
- `src/post.rs` + `assets/shaders/crt.wgsl` — CRT post-process pass
- `src/settings.rs` — load/save `settings.toml`

## Credits

- Font: [m6x11plus](https://managore.itch.io/m6x11) by Daniel Linssen
