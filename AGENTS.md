# drusty-pen

A local-first, hand-drawn-style whiteboard in Rust, in the spirit of Excalidraw. The GUI is
[rune](https://github.com/anomalyco/rune) (wgpu + winit). No online features: no collaboration
server, no sharing links, no AI.

## Core principles

Every change is judged against these four, in this order when they conflict:

1. **Beautiful visuals.** Crisp, smooth shapes at every zoom; calm, consistent chrome. Check
   every visual change in a screenshot before calling it done.
2. **Smooth interactions.** Feedback on the same frame as the input: drafts follow the pointer,
   modifiers apply the moment they're pressed, nothing janks or flickers.
3. **High performance.** Pan and zoom are live-only; a drag rebuilds only what it touches;
   nothing scales with document size per frame.
4. **Polish.** Hints, tooltips, shortcuts, edge cases (clicks, cancels, degenerate shapes)
   handled. Nothing half-finished ships.

- Feature checklist: [`EXCALIDRAW.md`](EXCALIDRAW.md). Tick items when they ship.
- Plan: [`milestones/`](milestones/README.md). Work one milestone at a time, in order.

## Layout

| Crate | Owns | May depend on |
|---|---|---|
| `crates/pen-core` | Elements, ids, `Document`, geometry, hit testing, commands, file format | std, serde, glam |
| `crates/pen-editor` | Tools, interaction state machines, selection, camera, history | `pen-core` |
| `crates/pen-render` | `Document` + editor overlay → `DrawList`; hand-drawn style; shape cache | `pen-core`, `pen-editor` |
| `app/drusty-pen` | rune window, panels, event and draw-list adapters | everything + `rune` |

**Only `app/drusty-pen` imports `rune`.** Rune is pre-1.0 and pinned by git rev. Every other crate
builds and tests with no window and no GPU.

Data flows one way: rune event → `InputEvent` → editor → `Command` → `Document` → `DrawList` →
`rune::Shape`. The `Document` is the only source of truth; nothing caches state it can derive.

## Commands

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p drusty-pen
```

Run all three checks before calling work done. The toolchain is pinned in `rust-toolchain.toml`.

Screenshot without a window: `cargo run -p drusty-pen -- --snap out.png --scale 1`. Rune's
`--script events.json --at SECONDS` drives real input first (see `app/drusty-pen/scripts/`).

## Rust guidelines

### Types
- Make illegal states unrepresentable: enums over flag combinations, `Option` over sentinels.
- Newtypes for ids and coordinate spaces (`ElementId`, `WorldPt`, `ScreenPt`). Never pass a bare
  `f32` pair across the camera boundary.
- Default to private, then `pub(crate)`. A `pub` item is an API promise.
- Derive `Debug` everywhere; `Clone, PartialEq` where cheap; `Copy` for small value types.
- Use builder methods or `..Default::default()` instead of long positional constructors.
- Colours are stored as sRGB. `rune::Color` is linear, so convert in the app adapter only.

### Errors
- Library crates: one `thiserror` error enum per module boundary. No `anyhow` outside the app.
- No `unwrap()` / `expect()` outside tests. If an invariant truly can't fail, use `expect("why")`
  and state the invariant.
- No panics in `pen-core` on user data. A corrupt file is an `Err`, not a crash.

### Ownership and state
- No `Rc<RefCell<_>>`, no `Arc<Mutex<_>>` for app state. State lives in one owned struct;
  handlers get `&mut`.
- Mutate the document only through `Command`s, so every change can be undone.
- Borrow in arguments (`&[T]`, `&str`), own in struct fields.

### Determinism
- The same document always renders the same pixels. Randomness comes only from an element's `seed`.
- Iterate in a stable order: `Vec`, `BTreeMap` or `IndexMap`. Never let `HashMap` order reach
  output, files or tests.

### Performance
- Don't allocate per frame in hot paths. Reuse buffers and cache per-element geometry by
  `(id, version)`.
- Pan and zoom never rebuild the scene; they move live values only.
- Measure with `--release` before optimising. Leave a comment with the numbers.

### Code shape
- `#![forbid(unsafe_code)]` in every crate except the app.
- Files stay under 600 lines. Split by capability (`tools/rect.rs`, `tools/arrow.rs`), not by kind
  of code (`types.rs`, `utils.rs`).
- Tests go in a sibling `tests.rs` or `*_tests.rs`. Every tool and command gets a test that
  drives it through `InputEvent`s, with no rune involved.
- Doc comments state invariants and units ("world points, y down"), not what the code obviously does.
- Add a dependency only when it replaces real work. Say why in the PR.

## Git

- Agents manage git: commit and push after every finished piece of work. No need to ask first.
- Small commits, one logical change each. Imperative subject, max. 72 chars
  (`Add rectangle tool`). Body says why, not what.
- Run fmt, clippy and tests before every commit. Never commit a red build.
- Work on `main` unless a change is risky or experimental; then use a branch and merge it back.
- Never force-push `main`, never rewrite pushed history, never commit secrets or `target/`.

## Done means

1. The milestone's "Done when" items can be shown working.
2. Tests cover the new behaviour; fmt, clippy and tests pass.
3. The milestone file and `EXCALIDRAW.md` are ticked.
4. It is committed and pushed.
