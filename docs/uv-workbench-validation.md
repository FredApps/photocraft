# 3D / UV update validation - 2026-10-27

Source baseline: official PhotoCraft v0.16.0,
`b1794c6e0608ccfc887213dcf58398977c5cb75e`, plus the local 3D/UV changes.
Checks ran on the Windows source checkout used for the working 0.16.0 build.
The publication checkout was compared with it; source differences are rustfmt and
the snapshot example's portable default output filename.

## Checks

| Check | Result |
|---|---|
| Native application, `cargo check -p photocraft --features uv-workbench --locked` | Passed |
| UV library unit tests | 6 passed, including FBX preflight rejection cases |
| Engine `uv_container` integration tests | 9 passed |
| CPU compositor `mask_paint` unit tests | 12 passed |
| UI library `uv_` test filter, with `uv-workbench` | 83 passed; 6 manual diagnostics ignored |
| All-target Clippy, `-D warnings`, UV/compose/GPU/engine/UI crates | Passed |
| Native UI check without the UV feature | Passed |
| UI check with UV for `wasm32-unknown-unknown` | Passed; six dead-code warnings in unchanged upstream files (five engine, one plugins) |
| `cargo xtask layers` | 32 crates, no violations |
| `cargo xtask scorecard --check` | Passed after regeneration for the added crate |
| Explicit rustfmt checks on changed Rust files | 113 files passed |
| Whitespace, presentation links and screenshot integrity | Passed; all four PNG SHA-256 hashes match the supplied originals |

The UI tests cover DX12/FXC startup, compositor fallback, original-resolution maps,
mask preview/commit, Fill transforms, image-shelf placement, mask-stack expansion,
painting/Undo, wireframe, navigation, workspace isolation and localization.
The existing offscreen mask-Levels/checker example also passed its interaction
assertions; its Properties render was inspected visually.

The upload preparation updates existing test calls for the UI context parameter,
uses the current U/V margin labels, and tests explicit mask-stack expansion and
collapse. Assertions remain enabled. Strict-lint cleanup names result/cache types,
groups viewport parameters and simplifies equivalent conditionals.

The full workspace/corpus sweep and performance benchmarks were not rerun for
this upload. These results do not claim that every workspace or integration test
is green.

## Upstream integration

A merge preview against upstream `main` at
`55b8df82fc74f01bfa24cef45599c603e6121dd0` reports conflicts, including moved or
removed UI modules. The update preserves the existing PR history and targets its
same branch; it still needs integration with current main before merge.
The checks above validate the 0.16.0 source, not an unresolved merge result.
