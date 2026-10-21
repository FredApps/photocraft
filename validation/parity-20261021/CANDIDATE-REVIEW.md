# Photocraft approved PR inventory and Photoshop-parity triage

Collected 21 October 2026; selected PR heads refreshed at 18:14 UTC / 20:14 Europe/Paris.

## Result and scope

Found **500 open, unmerged PRs whose latest decisive FredApps review is APPROVED**. Another **9** open PRs had earlier approvals superseded by CHANGES_REQUESTED; they are preserved in the excluded evidence instead of being counted as approved. The full reviewed population was 800 open PRs: 500 approved, 178 changes requested, 122 comment-only.

- [Complete 500-PR inventory](inventory.md): direct PR/review/CI links, approval freshness, merge state, triage band and holds.
- [Reviewed but not currently approved](reviewed-not-currently-approved.json): 300 PRs, including the 9 superseded approvals.
- approved-evidence-1.json through approved-evidence-5.json: 100 approved records each, with full head/approval SHAs, approval text, descriptions, checks, workflow runs, commit statuses and selected screening findings.
- [Selected source evidence](selected-source-evidence.json): deeper-check patches, human discussions, review threads, job details and relevant log excerpts.

“Unmerged” here means the pending **open** queue; abandoned/closed unmerged PRs are excluded. Nothing was merged, approved, closed, posted or changed on GitHub. No PR code or local tests were executed.

## Screening counts

| Measure | Count |
| --- | ---: |
| Approved review is on the current PR head | 414 |
| Approval predates current PR head | 86 |
| Latest upstream CI workflow on the current head succeeded | 437 |
| Upstream CI requires workflow approval | 53 |
| Upstream CI failed | 4 |
| Upstream CI in progress | 3 |
| No upstream CI workflow found for current head | 3 |
| Merge state clean / dirty / unstable / unknown | 286 / 93 / 42 / 79 |
| Same-head approval + successful upstream CI + clean merge state | 227 |

These dimensions overlap. The 227 are **screening candidates, not merge approvals**: #7318, #8012 and #8034 pass those mechanical checks but still have the concerns below. Approval-SHA mismatch means review freshness must be assessed; it does not by itself prove the added commits are faulty.

Verified exact-head green CI in FredApps' fork for three upstream action_required PRs: [#7904](https://github.com/FredApps/photocraft/actions/runs/39394831372), [#7240](https://github.com/FredApps/photocraft/actions/runs/39393188811), [#5327](https://github.com/FredApps/photocraft/actions/runs/39431080960). Fork CI is separate evidence, not a successful upstream check. The remaining 60 non-green/absent upstream cases have no matching green CI run in that fork in the fetched results; some have author or independent local-test reports.

Main moved during collection. The actual main ref read after the inventory was **6a2d027ee2e7ca12afd248544ed2262b0fa078dd**. Each record's base SHA is a PR API snapshot, not proof CI tested the present main+head merge. Recheck and test integration in small batches.

## Prioritize parity bug fixes

Every candidate below was open, approved at its unchanged current head, upstream-CI green, and cleanly mergeable in the selected refresh. These recommendations use the current description, FredApps review and CI; they are not new full source reviews of all 16 PRs.

| Order | PR | Why it comes first | CI |
| --- | --- | --- | --- |
| 1 | [#8042](https://github.com/storytold/photocraft/pull/8042) — Codecs: Fix corrupted CMYK JPEG exports (#8008) | Fixes corrupt CMYK JPEG output while preserving DPI in EXIF and retaining Adobe APP14; start with export correctness. | [green](https://github.com/storytold/photocraft/actions/runs/39494984316) |
| 2 | [#8250](https://github.com/storytold/photocraft/pull/8250) — fix(io): read a placed layer's custom warp over its source rect (#8227) | Fixes shifted/clipped PSD Smart Object contents caused by offset custom-warp bounds; includes import/export preservation tests. | [green](https://github.com/storytold/photocraft/actions/runs/39605316368) |
| 3 | [#8098](https://github.com/storytold/photocraft/pull/8098) — PSD layer comps: an entry without enab inherits the previous visibility, as in Photoshop (#8093) | Matches Photoshop layer-comp visibility inheritance for absent enab, including Last Document State handling. | [green](https://github.com/storytold/photocraft/actions/runs/39531055102) |
| 4 | [#7857](https://github.com/storytold/photocraft/pull/7857) — Replace Contents: contents of another size keep the placement's centre, as in Photoshop (#7844) | Keeps Smart Object placement centred when Replace Contents changes source dimensions. | [green](https://github.com/storytold/photocraft/actions/runs/39451683119) |
| 5 | [#6868](https://github.com/storytold/photocraft/pull/6868) — Keep shape fill and stroke alpha through PSD round-trip (#6180) | Preserves shape fill/stroke alpha through PSD round-trip; prevents document-fidelity loss. | [green](https://github.com/storytold/photocraft/actions/runs/39095489527) |
| 6 | [#6815](https://github.com/storytold/photocraft/pull/6815) — PSD: keep Photoshop 2026 Light adjustment layers verbatim and warn (#6542) | Preserves unsupported Photoshop 2026 Light adjustments verbatim with a warning, reducing destructive round-trip loss. | [green](https://github.com/storytold/photocraft/actions/runs/39624240474) |
| 7 | [#5607](https://github.com/storytold/photocraft/pull/5607) — PSD: PSB foreign-order long-key blocks keep 8-byte length in little-endian TIFFs (#5599) | Keeps PSB long-key block widths correct inside little-endian layered TIFF. | [green](https://github.com/storytold/photocraft/actions/runs/38802719135) |
| 8 | [#6244](https://github.com/storytold/photocraft/pull/6244) — Preserve opaque edges when placing scaled Smart Objects | Preserves opaque edges when placing scaled Smart Objects. | [green](https://github.com/storytold/photocraft/actions/runs/38778043020) |
| 9 | [#8005](https://github.com/storytold/photocraft/pull/8005) — Open: a file without type never waits for the font scan (#7332) | Prevents files without text from blocking on font scanning; type-bearing opens move the wait to a worker. | [green](https://github.com/storytold/photocraft/actions/runs/39624407480) |
| 10 | [#7894](https://github.com/storytold/photocraft/pull/7894) — Slice Select: Delete deletes the selected slice, not the layer (#7862) | Delete/Backspace in Slice Select deletes the slice rather than the image layer. | [green](https://github.com/storytold/photocraft/actions/runs/39365825839) |
| 11 | [#8053](https://github.com/storytold/photocraft/pull/8053) — Pencil: align 1 px click snapping with the line walker (#4895) | Aligns 1px Pencil clicks with line-walker snapping, including boundary tolerance. | [green](https://github.com/storytold/photocraft/actions/runs/39496855316) |
| 12 | [#8043](https://github.com/storytold/photocraft/pull/8043) — Keep Fixed Ratio marquee ratio when dragged past the canvas edge | Keeps Fixed Ratio marquee proportions at the canvas edge. | [green](https://github.com/storytold/photocraft/actions/runs/39492657980) |
| 13 | [#7919](https://github.com/storytold/photocraft/pull/7919) — Liquify: Alt-drag brush resize keeps the brush in place and no longer paints a dab (#7884) | Alt-drag Liquify brush resizing no longer moves the brush or paints a dab. | [green](https://github.com/storytold/photocraft/actions/runs/39386939655) |
| 14 | [#7968](https://github.com/storytold/photocraft/pull/7968) — fix(atn): a Group Layers step keeps its explicit layer through .atn (#7236) | Keeps explicit Group Layers action targets through ATN export/import. | [green](https://github.com/storytold/photocraft/actions/runs/39426498952) |
| 15 | [#7959](https://github.com/storytold/photocraft/pull/7959) — Actions: Emboss saves to and loads from .atn as Photoshop's Embs step (#7946) | Maps Photoshop Emboss actions to Embs with angle/height/amount. | [green](https://github.com/storytold/photocraft/actions/runs/39418443028) |
| 16 | [#7755](https://github.com/storytold/photocraft/pull/7755) — fix(actions): refuse lossy seeded Crystallize ATN export | Refuses seeded Crystallize settings that ATN cannot faithfully represent, rather than silently changing playback. | [green](https://github.com/storytold/photocraft/actions/runs/39251360334) |

## Recommend holding or separating from the parity batch

### #7318 — shortcut defaults: do not merge as proposed

Head a3af8d2691b367806239048b82fa9a242f232b8f. [FredApps approval](https://github.com/storytold/photocraft/pull/7318#pullrequestreview-5584758977) explicitly leaves the parity departure for maintainers. The diff changes General Preferences from Cmd+K to Cmd+comma, adds Cmd+K as a Crop alternate, and removes the Hide Layers shortcut. It also replaces the previous visibility-toggle regression with a test that expects the new behavior.

Adobe's [current Preferences documentation](https://helpx.adobe.com/ie/photoshop/desktop/get-started/settings-and-preferences/adjust-preferences.html), updated 28 September 2026, still assigns Ctrl+K / Command+K to Preferences. Thus CI success confirms tests accepting a deliberate departure. Preserve Photoshop defaults; offer an optional macOS-convention shortcut set if wanted.

### #6880 — single Smart Object conversion: hold the removed nesting workflow

Head 2466f5c3a74b62b5926951d137965be513318a4f. [Production diff](https://github.com/storytold/photocraft/blob/2466f5c3a74b62b5926951d137965be513318a4f/crates/engine/src/smart_cmds.rs) adds convertible_to_smart and refuses converting an already-smart single target; the pre-existing direct nesting MCP test is rewritten to edit contents and nest there. Multiple selections can still nest Smart Objects. Adobe documents [nested Smart Objects](https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/update-linked-smart-objects.html); that source establishes nesting, not every current single-layer menu case.

This is a likely workflow parity regression requiring a current Photoshop reproduction of single embedded and linked Smart Objects before removing the existing path. Keep the direct conversion/nesting regression, including transforms and filters, instead of only asserting the new disabled state. Upstream CI is action_required; the approval cites Linux testing without Windows or Clippy.

### #8012 — Liquify freeze: hold for a missing before/after invariant

Head d470faa65fabe5d674790062cc8d537e0fb15cf4. The [source](https://github.com/storytold/photocraft/blob/d470faa65fabe5d674790062cc8d537e0fb15cf4/crates/algo/src/liquify.rs) changes cubic displacement to bilinear wherever a fully frozen mask applies. The author explicitly measures about **0.0675 source pixels** of image shift when freezing an already-warped image. The new test records its “before” only after applying the freeze, so it cannot detect that transition.

Adobe describes [freeze masks as protection from distortion](https://helpx.adobe.com/uk/photoshop/desktop/effects-filters/artistic-stylize-filters/freeze-or-thaw-areas.html). The concrete code/test gap is established; exact numerical Photoshop freeze/thaw behavior was not reproduced here. Require a warp → capture → freeze → capture → thaw sequence with no further warp to preserve the image, plus the existing adjacent-warp test. Preserve the existing mapping rather than using mask application to change interpolation.

### #8034 — Windows save durability: hold for native Windows CI execution

Head 851ca348a71ff7063e4b7ad7609d290500845c2c. [CI run](https://github.com/storytold/photocraft/actions/runs/39484506477) is green, but [test (windows-latest)](https://github.com/storytold/photocraft/actions/runs/39484506477/job/118851825864) ran for **3 seconds** and contains only setup, the aggregate-status shell check and cleanup. There are no Windows unit/integration legs in that run. It is an Ubuntu gate, not native Windows tests.

The [workflow platform selector](https://github.com/storytold/photocraft/blob/main/.github/workflows/ci.yml) does not automatically request Windows for format-only changes. Require native Windows unit and integration jobs, including a_failed_publication_flush_is_reported_and_can_be_retried, the directory handle/flush regression and durability_tests, at this head. The author's substantial Windows VM power-cut evidence is useful and remains credited; it does not make the hosted Windows-only regressions execute. This is a CI coverage hold, not a claim the durability fix is wrong.

### #3283 — Smart Filter blending: split safe fixes from known approximation

Head 2a81d271b5a28bff8fff62645b1b6ecdbf03bd59. Current source/docs explicitly describe Dissolve as blending without Photoshop's dither; changed-alpha non-Normal blending uses source-over fallback semantics. [Adobe's Dissolve definition](https://helpx.adobe.com/in_hi/photoshop/desktop/repair-retouch/adjust-light-tone/blending-mode-descriptions.html) specifies random pixel replacement according to opacity, so the documented approximation is a real behavior gap. Green CI cannot establish parity for behavior intentionally omitted.

Recommend splitting the useful editor/alpha fixes from the approximation, or implementing a deterministic coverage rule and Photoshop oracle tests before marking blending parity complete. The PR is also conflicting with main. An unresolved bot thumbnail-cache thread is preserved in source evidence, but was not promoted to a fresh confirmed defect here.

### #7904 — Halftone Pattern: hold exact-parity signoff pending Photoshop fixtures

Head 69af55ff7e6b8c5bccaefabc04f851f567162905. The tone/coverage tests and [exact-head fork CI](https://github.com/FredApps/photocraft/actions/runs/39394831372) are green. The description expressly says Size spacing and Circle/Line patterns were not compared with Photoshop. Tests of mean ink are useful mathematical invariants, but do not establish the target application's screen frequency or output. Require small Photoshop-authored fixtures for Dot/Circle/Line and representative Size/Contrast settings. This is an oracle gap, not proof its output is wrong.

### #5145 — ML Subject / Generative Fill: hold broader rollout

Head bdb7e062899e742c82f9f6c29b7e61ff30361101. The [approval](https://github.com/storytold/photocraft/pull/5145#pullrequestreview-5545382186) records a model test returning early without the model, no live provider checks, and untested Windows/macOS credential behavior. There is no upstream CI run for the current head, and the fresh metadata confirms conflicts. Require pinned real-model CI (no skip), refreshed integration, relevant platform persistence tests and an explicit provider integration validation plan. The PR itself also cites unresolved product decision #41. Earlier credential-isolation and persistence findings were fixed; do not reopen them without new evidence.

### #3342 and #7970 — defer broader extensions, rather than calling them bug fixes

[#3342](https://github.com/storytold/photocraft/pull/3342), head 962c92c7016cf31acd604898196dcb555012ca74, labels its live-type Distort/Perspective/free Warp behavior **“Better Than PS”** in source. It removes deliberately Photoshop-compatible disabled rules. Its PSD adapter exports distorted type as a Smart Object; re-import leaves it a Smart Object. That is a deliberate product extension with a changed round-trip editing model. Keep it out of a parity bugfix batch unless that extension/PSD contract is explicitly wanted.

[#7970](https://github.com/storytold/photocraft/pull/7970), head 9555cf2b4a5a3c9021e6a9bdfc2e66d7f704491b, is the responsive-layout feature stack's model/schema foundation. It unconditionally writes native format v2, making even re-saved nonresponsive documents unreadable by v1-only readers. Its earlier correctness findings were fixed, CI is green and it is cleanly mergeable. Recommend a separate format-compatibility/feature decision; this is prioritization, not a newly discovered Photoshop rendering defect.

## Existing CI failures that should stop immediate merging

| PR | Current evidence | Next step |
| --- | --- | --- |
| [#6384](https://github.com/storytold/photocraft/pull/6384) | [Ubuntu log](https://github.com/storytold/photocraft/actions/runs/38825328332/job/116758008895): ui_set_description_names_every_accepted_field failed; 863 passed, 1 failed, most tests not run after fail-fast. Windows Test also failed. | Refresh and fix/retest; this audit did not establish causality against an identical baseline. |
| [#6156](https://github.com/storytold/photocraft/pull/6156) | [Clippy log](https://github.com/storytold/photocraft/actions/runs/39002946363/job/117309524489): E0063 missing export_presets in Persisted initializer; several downstream jobs fail. | Integrate current main's fix and rerun; do not describe the old red run as passing. |
| [#3535](https://github.com/storytold/photocraft/pull/3535) | [Ubuntu log](https://github.com/storytold/photocraft/actions/runs/39633543371/job/119337943701): contained_panics_count_and_graceful_errors_do_not fails; 4377 passed, 1 failed, 5602 not run. Approval also predates current head. | Refresh approval and resolve/retest failure. |
| [#6896](https://github.com/storytold/photocraft/pull/6896) | Latest [CI](https://github.com/storytold/photocraft/actions/runs/38990679218) is failure with zero jobs returned; documentation also failed; branch conflicts. | Diagnose workflow/admission failure before inferring a code failure. The approved heuristic also needs real-photo evaluation. |
| [#2868](https://github.com/storytold/photocraft/pull/2868) | Core CI is green, but [FreeBSD](https://github.com/storytold/photocraft/actions/runs/39624762621/job/119307074641) fails an_ambient_map_or_lut_never_renders_unfiltered_pixels_into_the_stack_cache in smart_filter_authorization. Approval predates current head. | Diagnose against baseline and refresh review. This log does not establish a JPEG XL decoder defect. |

## Suggested landing discipline

Start with file fidelity and incorrect editing results, then narrow interaction fixes, then measured performance. Preserve unknown Photoshop data with explicit warnings. Keep non-Photoshop extensions, schema changes, packaging and documentation separate in priority.

Land small independent batches and validate the resulting integration against current main. A check named Windows may be an aggregate; a green test may return early without its model/font/GPU/corpus; CPU/GPU agreement may still disagree with Photoshop. Accept only evidence for the platform, feature inputs and behavior actually exercised.

## Limits

All 800 review histories and all 500 approved PR metadata/check/workflow/status snapshots were gathered; search pages reported incomplete_results=false, and no review endpoint reached the 100-review page limit. Deeper source/discussion review was selective. The remaining PRs need individual source, complete discussion, unresolved-thread and integration review before merging. No new GitHub reviews, comments or check reruns were submitted. No local or remote PR test workload was launched.
