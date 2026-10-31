# M0: Skeleton and camera spike

**Goal:** the workspace builds, a rune window opens with an empty white canvas, and we know how
the camera will work.

**Estimate:** 30–60 min.

## Done when

- [x] `cargo run -p drusty-pen` opens a window with a full-size canvas.
- [x] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- [x] The camera decision below is filled in.

## Tasks

1. [x] Root `Cargo.toml`: workspace, `resolver = "3"`, edition 2024, shared `[workspace.lints]`
       (`unsafe_code = "forbid"` for libraries, clippy `all` + selected `pedantic`).
2. [x] Pin rune in `[workspace.dependencies]`:
       `rune = { git = "https://github.com/anomalyco/rune", rev = "791971423cf6614aff061491e1f82e283dfc9f0e" }`.
3. [x] Create empty crates `crates/pen-core`, `crates/pen-editor`, `crates/pen-render` with `lib.rs`
       and one smoke test each.
4. [x] Create `app/drusty-pen` with `rune::run(AppSpec::new(...))`, one `canvas()` filling the
       window, drawing a single test rectangle.
5. [x] Add a headless app test using `rune::Runtime` (pattern: rune's `examples/sketch`).
6. [x] **Camera spike** (time-box 30 min), answer:
       - Does `Shape` draw points with negative or out-of-bounds coordinates?
       - Does `live_offset` + `live_scale` on a wrapper give a correct pan/zoom about the cursor
         (`live_scale` scales about the node centre)?
       - Do `on_drag` / `on_wheel` coordinates come back in pre-scale (world-ish) points?

## Camera decision

- [x] **A: rune-side camera.** World container with `live_offset` / `live_scale`, `tx.live_only()`
      on pan/zoom. Scene never rebuilds while panning. Preferred.
- [ ] **B: app-side camera.** `pen-editor` transforms world → screen; the scene `Shape` is rebuilt
      on every camera change. Fallback if A can't draw an unbounded world.

Chosen: **A**. Reason: the spike answered all three questions in A's favour.

1. **Out-of-bounds drawing works.** A `canvas` laid out as a zero-size node draws its `Shape` at
   negative and far-off coordinates with no clipping. That makes the node a pure transform.
2. **Zoom about the cursor works.** `live_scale` scales about the node centre. For a zero-size
   node at (0, 0) the centre is the origin, so the presented transform is
   `screen = world * zoom + offset`. With `screen = (world - scroll) * zoom` that gives
   `offset = -scroll * zoom` (`app/drusty-pen/src/present.rs`). The headless test checks that
   rune's drawn rect matches `Camera::world_to_screen`.
3. **Event coordinates.** `on_wheel` and `on_magnify` sit on the *unscaled* viewport, so they
   get viewport points. The editor's `Camera` converts them to world points. (A handler on the
   scaled node would get points divided by its scale, per rune's docs. We don't rely on that.)

Pan and zoom call `tx.live_only()`. The tests assert `stats.commits` stays the same, so the
scene `Shape` is never rebuilt while the camera moves.

## Notes

- Linux needs Vulkan drivers plus Wayland or X11 dev libraries for wgpu/winit.
- Rune at the pinned rev does not build on Rust 1.93: it uses `AtomicUsize::try_update`. The
  toolchain is pinned to 1.98.1 in `rust-toolchain.toml`.
- `rune::Color` is **linear** sRGB. Writing `#1e1e1e`'s bytes straight into its fields renders
  mid grey. Store sRGB in the document (as Excalidraw does) and convert in the app adapter
  (`palette::srgb`, or `rune::color::hex`).
- Stroke widths in the world `Shape` scale with zoom (2 pt at 300% draws 6 px). That matches
  Excalidraw, where stroke width is in world units.
- Headless screenshots: `cargo run -p drusty-pen -- --snap out.png --scale 1`, optionally with
  `--camera X,Y,ZOOM` or `--script app/drusty-pen/scripts/pan-zoom.json --at 0.5`.
- Window resize does not re-centre the camera. Nothing needs it yet.
