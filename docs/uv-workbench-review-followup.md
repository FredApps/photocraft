# UV workbench review follow-up

This note tracks the finalized CodeRabbit review findings for PR #8805 and the
focused validation run for their fixes. It complements
[`uv-workbench-validation.md`](uv-workbench-validation.md), which records the
broader merge validation and its previously accepted pixel-test failures.
The review is available at [PR #8805](https://github.com/storytold/photocraft/pull/8805).

## Review findings

| # | Finding | Resolution |
|---|---|---|
| 1 | Unbounded mask refresh could enumerate oversized or invalid persisted Fill bounds. | Bound unscoped Fill expansion by checked area and tile budgets; canvas consumers refresh only the canvas; reject malformed bounds during load. |
| 2 | Cache-only mask edits could diverge from the editable mask-paint stack. | Validate edits on a clone and reject cache-only stack mutations atomically; reject unsupported stack transforms before mutation. |
| 3 | Removing a stack layer below the active layer left the active index pointing at the wrong layer. | Adjust the active index when deleting a preceding layer and cover it with an engine regression. |
| 4 | A 64-megapixel Fill could allocate multiple full-resolution RGBA buffers. | Keep the supported size while rendering in bounded horizontal slabs and writing each completed region. |
| 5 | The live-fill snapshot example assumed a developer-specific drive path. | Accept an explicit path and use a portable relative default. |
| 6 | The mask-paint snapshot example assumed developer-specific input/output paths. | Accept explicit paths and use portable relative defaults. |
| 7 | The standalone UV renderer did not observe GPU device loss. | Attach a device-health watcher when no shared application GPU health handle is available. |
| 8 | Deferred GPU redraws could drop materialized CPU mask edits or repeat full document cloning. | Preserve materialized masks in deferred work; materialize only when a CPU fallback is actually required and use canvas-bounded refresh. |
| 9 | Paint picking ignored sticky modifier state. | Include sticky modifiers in the picker-suppression gesture check. |
| 10 | A stale opacity preview error prevented recovery writes for other dirty documents. | Retain the preview error while continuing canonical recovery saves for other dirty documents. |
| 11 | Alt-clicking a Gray mask view could immediately re-enable the view being toggled off. | Match the active Gray, unfiltered view to the exact layer before suppressing re-enable; add a layer-identity regression. |
| 12 | Control-right brush resizing leaked from the UV workspace into ordinary 2D painting. | Scope the normalized Control-right gesture to UV workspace; retain Alt-right behavior elsewhere. |
| 13 | Transient fit/fill pending flags made a clean UV project appear dirty. | Ignore those transient flags only in the dirty-state comparison; retain them in serialization and save/restore. |
| 14 | Validation documentation described intermediate merge state and linked local logs. | Align the validation note with the finalized merge state and portable references. |
| 15 | Invalid geometry construction permanently discarded the UV renderer. | Keep the existing renderer available so a later valid model can recover. |
| 16 | Several UV workspace labels were hard-coded and missing Russian localization. | Route the labels through translation keys and add Russian catalog entries. |

## Focused validation

The focused checks passed:

- `cargo test -p photocraft-engine --features uv-workbench --test uv_mask_paint` — 8 passed.
- `cargo test -p photocraft-compose --lib unbounded_refresh_skips_pathological_persisted_fill_bounds` — 1 passed.
- `cargo test -p photocraft-format loader_rejects_inverted_fill_and_bounds_on_non_fill_layers` — 1 passed.
- `cargo test -p photocraft-ui-egui --features uv-workbench stale_opacity_preview_fails_recovery_without_losing_pending_value` — 1 passed.
- `cargo test -p photocraft-ui-egui --features uv-workbench uv_container_save_quit_and_workspace_scope_preserve_normal_canvas` — 1 passed.
- `cargo test -p photocraft-ui-egui --features uv-workbench alt_mask_view_toggle_only_suppresses_reenable_for_the_same_gray_layer_mask` — 1 passed.
- `cargo test -p photocraft-ui-egui --features uv-workbench mac_control_right_brush_resize_is_scoped_to_the_uv_workspace` — 1 passed.
- `cargo clippy -p photocraft-compose -p photocraft-engine -p photocraft-format -p photocraft-ui-egui --all-targets --features photocraft-ui-egui/uv-workbench -- -D warnings` — passed.
- `cargo check -p photocraft-ui-egui --all-targets` — passed.
- `cargo test -p photocraft-engine --test panic_hunt -- --ignored` — adversarial command panic hunt passed (1 test).
- `cargo xtask layers` — passed, 32 crates with no layering violations.
- `cargo xtask wasm` — passed for 25 crates and the codecs `heif` feature check.

These focused checks do not replace the broader validation documented in
[`uv-workbench-validation.md`](uv-workbench-validation.md). The six previously
recorded pixel assertions remain deferred.
