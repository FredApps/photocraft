# Windows graphical verification — approved parity integration

Status: **NOT RUN**. This document is a protocol, not a visual test result.

## Candidate and scope

Use the accompanying source-commit.txt, source-tree.txt, SHA256SUMS.txt and integration-plan.json. The package is an unsigned Windows x64 release validation build from FredApps/photocraft. Its executable version and Help > System Info must identify the packaged source commit. Baseline source is 6a2d027ee2e7ca12afd248544ed2262b0fa078dd. Every included upstream head must be an ancestor of the candidate; integration-receipts.json records each merge.

15 PRs are integrated. PR 6868 was removed from the originally proposed 16 after fresh discussion showed unresolved alpha/opacity defects. The earlier hold list (7318,6880,8012,8034,3283,7904,5145,3342,7970) is not added by this integration. An excluded head need not be absent from all ancestry if upstream later merges it: compare the pinned baseline and explicit integration receipts.

PR 5607 is additive regression coverage: its implementation fix already exists on the baseline. PR 6815 preserves unsupported data and warns; it does not implement Light rendering. PR 8043 has an unresolved fractional-edge rounding risk and must be tested explicitly before parity signoff.

## 1. Admission and identity

1. Download the artifact from the recorded GitHub run and extract to a fresh test directory on the designated Windows verification machine. Do not overwrite an installed PhotoCraft build or personal settings. Run as a standard user. Keep a pristine copy of every input fixture.
2. Verify ZIP and executable SHA-256 hashes against the build receipt. Record full commit, tree, artifact/run URL, file hashes and UTC test start. Verify the candidate through Help > About/System Info as well as the manifest. Do not accept a screenshot from another executable path.
3. Confirm native Windows unit and integration CI actually ran for this exact source/tree; the three-second aggregate Windows check is insufficient. Preserve Linux, corpus, font/no-font variants, Clippy, layering and WASM outcomes. No zero-match, silently skipped corpus/font/model/GPU result counts as exercised coverage.
4. Record Photoshop's exact version/build from Help > System Info, Windows edition/build, GPU/device/driver/backend, display resolution, per-monitor scaling, HDR state, ICC display profile, input device and installed fixture font. Keep Photoshop's system-info report private if it contains personal paths.
5. Use a new PHOTOCRAFT_CONFIG_DIR for the candidate; launch the baseline with a different directory if doing a baseline comparison. Do not reset the user's Photoshop preferences: record them and use disposable documents.
6. Set the same image ICC profile, proofing state, rendering intent, black-point compensation, bit depth, image size and resampling method in both apps. Turn soft proofing/HDR off for ordinary SDR comparisons, or record a separate HDR test. Preserve alpha when exporting lossless references.
7. Read known risks below before testing. Do not mark an intentional unsupported feature as rendered parity.

## 2. Evidence and verdicts

For each case retain: input hashes, exact steps, screen recording or before/after PNG screenshots, Windows scaling/backend, actual result, supporting native-test name/log, saved output documents and Photoshop reference. File naming: CASE-step-app-scale-backend.png; for example G02-after-edit-photocraft-150pct-dx12.png.

Allowed results: NOT_RUN, PASS, FAIL, BLOCKED_FIXTURE, BLOCKED_ENVIRONMENT, NOT_EXERCISED. A missing PSD or inability to make the relevant font-scan/GPU path run is not PASS. Record observations and link evidence before changing a result. Retesting creates a new attempt; keep the first failure.

Capture the whole window once to show app identity/settings, then tight 100%/800% image crops where relevant. Use Windows Snipping Tool or a trusted desktop recorder for actual displayed frames. An offscreen egui snapshot complements a native-window screenshot; it does not prove native DPI, keyboard, file-dialog or driver behavior.

Compare appearance using registered lossless image exports at native pixel size in addition to screenshots. For geometry, layer visibility, lossless source identity and actions parameters use exact expectations. For JPEG, interpolated warps, CMS and filter output set an explicit reference/tolerance per case before judging; record max/channel difference and changed-pixel count. If no justified tolerance exists, report the measurement and keep numerical parity unverified. Do not compare whole-window Photoshop and PhotoCraft screenshots pixel-for-pixel.

On failure save the original input, actual output, screen recording, crash/recovery logs and minimal steps. Repeat once from a clean launch and, where useful, the pinned baseline. Do not change fixtures or defaults to hide a failure. Separate a pre-existing baseline defect from an integration regression, but retain a parity hold when the target behavior is still wrong.

## 3. Fixture preparation

Run the trusted make_fixtures.py supplied with this protocol to generate original neutral PNG inputs and hashes. It requires Python's standard library, not PhotoCraft source execution. Complex PSD/PSB/ATN fixtures are authored in the available Photoshop instance using the recipes below, or obtained from the linked original bug report and normalized there. Never overwrite downloaded originals.

Set Photoshop's PSD/PSB maximize-compatibility option consistently and record it. For each authored fixture save a Photoshop-rendered PNG reference, the editable document and the exact settings. Use a neutral installed font for text tests; record its family/version without copying font files into this bundle.

The issue-8227 reporter warns that the attached PSD's cached composite is already wrong. Photoshop must re-render and save that fixture before its pixels are used as the oracle. A default custom warp whose bounds start at the origin does not exercise the bug.

## G01 — CMYK JPEG interoperability and resolution (PR 8042)

Source: https://github.com/storytold/photocraft/pull/8042

**Fixture:** Open grid-512.png, convert a duplicate in Photoshop to CMYK using an installed CMYK profile, set 300 ppi without resampling, and save a layered PSD. Keep RGB and grayscale copies as controls. Do one set with EXIF camera data and one without.

1. Open the CMYK PSD in PhotoCraft; confirm mode, embedded profile and 300 ppi.
2. Save a Copy as JPEG at a recorded quality. Open that JPEG in Photoshop and Windows Photos.
3. In Photoshop, inspect Image Size with Resample off; inspect Image > Mode and the assigned profile. Compare the JPEG against the source in the same CMYK space.
4. Repeat RGB and grayscale JPEG exports; include an asymmetric resolution fixture if available.

**Pass gate:** JPEG opens in both applications, stays CMYK, and retains 300 ppi. No corrupt-file dialog or inverted channels. RGB/grayscale controls keep their prior behavior. JPEG is lossy: compare against an agreed same-quality reference; do not demand byte equality.

**Retain:** CMYK mode/profile and Image Size screenshots, Windows Photos screenshot, exported JPEG, source PSD, and JPEG marker/EXIF native-test results.

## G02 — Offset custom warp survives forced re-render (PR 8250)

Source: https://github.com/storytold/photocraft/pull/8250

**Fixture:** Use the original repro.psd from issue 8227 (download link in fixture-sources.json). Open it in Photoshop first, perform a harmless contents edit, save as ps-reference-warp.psd, and close. The reporter notes that repro.psd's initial composite is already wrong; it cannot serve as the expected image.

1. Capture Photoshop at 100% and export the normalized reference as PNG.
2. Open ps-reference-warp.psd in PhotoCraft. Edit Contents, invert twice (or add/delete an empty layer), save contents and return to the parent. This must force a re-render.
3. Replace Contents with the supplied same-size 2850×2850 grid and record the result separately.
4. Save a new PSD, close, reopen in PhotoCraft, then open it in Photoshop; force the same harmless contents edit in Photoshop.
5. Repeat with an unwarped Smart Object and an identity warp as negative controls.

**Pass gate:** Grid and magenta border fill the intended curved frame before and after re-render; no 15%/24% shift or right/bottom clipping. Placement, source size and editability survive the round trip. Compare registered lossless exports, not only the initial cached preview.

**Retain:** Before/after 100% crops, visible four border corners, Photoshop reference PNG, candidate PNG, round-tripped PSD, and a difference image.

## G03 — Photoshop layer-comp visibility inheritance (PR 8098)

Source: https://github.com/storytold/photocraft/pull/8098

**Fixture:** Author the two minimal PSDs described in issue 8093: (A) add layer C after comp c1, recapture c1 with C visible, save with C hidden; (B) A visible in c1, hidden in c2, unchanged hidden in c3, save with c1 applied. Add a position-only comp as a control.

1. Open each PSD fresh in Photoshop and record each comp's eye states and rendered result.
2. Open fresh in PhotoCraft; apply c1 in A: C must become visible. Apply c3 directly in B: A must become hidden.
3. Apply c1→c3, c2→c3 and c3→c1 in different orders; close/reopen between direct-apply checks.
4. Apply the position-only comp after changing visibility by hand; it must leave visibility alone.
5. Save/reopen in both applications; check Last Document State is not polluted by inherited comp visibility.

**Pass gate:** Every leaf-layer eye state matches Photoshop for each visibility comp, regardless of prior comp. Position-only comps do not gain visibility behavior. Existing Last Document State remains separate.

**Retain:** Layer Comps and Layers panel screenshots together, a per-layer visibility matrix, both source PSDs and round-tripped PSDs.

## G04 — Replace Contents keeps centre and scale (PR 7857)

Source: https://github.com/storytold/photocraft/pull/7857

**Fixture:** Use red-200.png and blue-400x200.png. Create 1000×1000 Photoshop and PhotoCraft documents, place red as an embedded Smart Object centred at (500,500), with fit disabled and scale 100%.

1. Record transform bounds, centre and scale. Replace with blue-400x200.png.
2. At 100%, expect bounds x=300,y=400,w=400,h=200 (centre 500,500). Undo and redo.
3. Repeat with a 150% placement; allow the documented raster-boundary rounding, but require the same geometric centre and scale.
4. Replace again with a same-size image: no further translation. Repeat with rotated/skewed and perspective placements.
5. Repeat with two shared instances. Test linked-source access only inside the approved fixture folder; denied/unavailable old-source behavior is a separately documented fallback.
6. Save PSD and compare/reopen in Photoshop.

**Pass gate:** Ordinary replacements grow/shrink around the original centre without losing scale/rotation/perspective; undo restores the old contents and placement. Warped/stacked placements are explicitly outside the recentering change: record them as controls, not proof of the general claim.

**Retain:** Properties/Transform bounds before and after, PSD round trips, undo/redo captures and perspective screenshots.

## G05 — Unsupported Photoshop Light data is preserved with a warning (PR 6815)

Source: https://github.com/storytold/photocraft/pull/6815

**Fixture:** In Photoshop 2026 create a Light adjustment over grid-512.png. Use Exposure -2.30, Contrast -2, Highlights -64, Shadows 0, Whites -47, Blacks -36 where supported. Save layers to PSD. Also create a conventional Brightness/Contrast PSD.

1. Capture the six Light values and source appearance in Photoshop.
2. Open in PhotoCraft; capture the import warning identifying Light/the layer. Do not edit that adjustment.
3. Save As a new PSD, close, reopen the new file in Photoshop, and inspect the six Light values and mask.
4. Compare Photoshop's before/after lossless renders. Repeat with a benign unrelated layer edit if supported.
5. Open the conventional Brightness/Contrast control: it must still import as an editable supported adjustment.

**Pass gate:** The unsupported warning is visible, Light settings/mask remain intact after return to Photoshop, and no Light layer is silently replaced with incomplete Brightness/Contrast. PhotoCraft rendering the unsupported Light effect is NOT a pass criterion; this change intentionally does not implement it.

**Retain:** Warning screenshot, Photoshop six-value panels before/after, source and saved PSDs, and byte-preservation native test.

## G06 — Layered TIFF/PSB foreign byte-order long blocks (PR 5607)

Source: https://github.com/storytold/photocraft/pull/5607

**Fixture:** The PR adds a synthetic wire-format regression; the implementation fix is already in main. Retain a generated foreign-order long-key layered TIFF/PSB test fixture from the native test job if available, with the following lyid block. Add ordinary Photoshop-created PSD, PSB and layered TIFF controls.

1. Require a native Windows log showing psb_foreign_order_long_key_block_keeps_its_8_byte_length actually passed.
2. Open the structural fixture and controls in PhotoCraft; inspect layers and image.
3. Save a copy and open in Photoshop; verify layers and identity-sensitive selection/round trips.
4. Retain the wire-byte assertion proving an eight-byte Mt16 length and intact following block; a screenshot alone cannot prove it.

**Pass gate:** Exact native wire-format test passes; files open without block desynchronization, missing following layers/IDs or unexplained flattening. If a structurally valid target-specific file is unsupported by Photoshop, record that separately instead of pretending a visual pass.

**Retain:** Native test line/count, structural fixture hash, screenshots of Layers panels in both apps and round-tripped files.

## G07 — Opaque Smart Object edges stay opaque when downscaled (PR 6244)

Source: https://github.com/storytold/photocraft/pull/6244

**Fixture:** Use opaque-grid-512.png, a high-contrast opaque border on a transparent parent canvas, and soft-edge-control.png. Prepare 8-, 16- and 32-bit documents where the operation is supported.

1. Place the opaque image at 50%; zoom to 800% and inspect all four corners over a checkerboard and solid contrasting background.
2. Force a contents re-render, then apply Gaussian Blur as a Smart Filter at recorded radii.
3. Compare the equivalent Photoshop stack, with matching zoom/interpolation and backgrounds.
4. Repeat at 100% as a control and with the genuinely soft-edge source. Save/reopen PSD.

**Pass gate:** No artificial partly transparent border appears from sampling beyond an opaque source. All four edge/corner positions retain the expected coverage, including after re-render/blur. A source that truly has transparent/soft edges must keep them.

**Retain:** 800% edge crops in both apps, full-image view, PSD and lossless alpha-bearing exports at each supported depth.

## G08 — Opening images does not block on unrelated font scanning (PR 8005)

Source: https://github.com/storytold/photocraft/pull/8005

**Fixture:** Use no-type.psd and text-in-group.psd, plus a large installed font collection or a fresh isolated app profile. Do not install/uninstall system fonts just to force a delay. Keep startup versus font-scan duration distinguishable.

1. Close all candidate instances and launch with a fresh test profile while recording the screen.
2. During an actual font scan, open the no-type PSD. Record request-to-first-image and whether the UI remains responsive.
3. Repeat with the text-in-group file, including a missing-font example. Observe Loading fonts/progress and cancellation.
4. Repeat a warm-profile control; compare equivalent opening in Photoshop for behavior, not a fixed speed ratio.
5. Require the native a_document_without_type_never_waits_for_the_text_engine test as deterministic coverage if a real scan cannot be caught.

**Pass gate:** A document without type is not held behind text-engine locking. A text document waits on its worker, reports progress, and can cancel without ghost documents or partial replacement. No real scan means the graphical scan condition is NOT_EXERCISED, not PASS.

**Retain:** Screen recording and timings, no-type/text fixtures, font/profile context and native lock-regression log.

## G09 — Slice Select deletion never deletes the image layer (PR 7894)

Source: https://github.com/storytold/photocraft/pull/7894

**Fixture:** Create two raster layers, at least two user slices, and an auto slice. Save a source PSD; show Layers panel and slice overlays.

1. With Slice Select active, select a user slice and press Delete; undo and repeat with Backspace.
2. Confirm only that slice disappeared, with image-layer count and pixels unchanged.
3. Lock slices and repeat; deletion must be refused without deleting layers.
4. Select an auto slice and press Delete/Backspace; neither the image layer nor an unrelated user slice may be deleted.
5. Focus a text entry and type Delete/Backspace; it must edit text instead of slices. Switch documents and check selection ownership before deleting.

**Pass gate:** User slice deletion uses one undoable slice operation; locked/auto/text-focus cases preserve the document layers. Cross-document selection must not delete an unintended slice; record any existing selection-model issue separately.

**Retain:** Before/after slice overlays and layer count, Undo history screenshots and a recording of both keyboard keys.

## G10 — One-pixel Pencil click/Shift-line agreement (PR 8053)

Source: https://github.com/storytold/photocraft/pull/8053

**Fixture:** Create a 64×64 transparent RGB/8 document, Pencil size 1 px, hardness 100%, opacity 100%, smoothing/stabilization and symmetry off. Repeat selected checks at other supported depths.

1. At 1600%, click selected pixel centres and corners; Shift-click to join the clicked endpoints.
2. Compare against Photoshop Pencil with identical settings; inspect at native 100% and high zoom.
3. Repeat horizontal, vertical and diagonal segments in every direction and at multiple Windows scaling factors.
4. At non-integer zoom (for example 125%) repeat near pixel boundaries. The 5e-8 versus 2e-7 numerical boundary needs the native test; mouse interaction alone cannot establish it.
5. Undo the line and confirm the preceding click remains exactly as expected.

**Pass gate:** The click and line walker agree on the endpoint pixel; no split click, adjacent stray pixel or missing endpoint. Lossless pixel masks match the intended discrete path.

**Retain:** Nearest-neighbour enlarged PNGs, native-size PNGs, tool settings screenshot and both named Shift-click regression results.

## G11 — Fixed Ratio marquee remains proportional at canvas edges (PR 8043)

Source: https://github.com/storytold/photocraft/pull/8043

**Fixture:** Create a 400×300 image. Use Rectangular Marquee, Fixed Ratio 2:1, then 1:1. Show the Info panel. Include a fractional document-coordinate start by using non-integer zoom.

1. Drag a 2:1 selection past right/bottom edges; include the integer-start 300×150 regression case. Compare Photoshop.
2. Repeat into all four corners, with Alt-centred drawing and an axis with no movement.
3. Switch to 1:1 and reproduce a start near (100.2,100.2), with the bottom edge limiting the drag. Measure the committed selection after release, not only the preview.
4. Explicitly look for a 201×200 result caused by post-rounding clipping. Repeat Elliptical Marquee and Shift-square as controls.
5. Undo/reselect, save/reopen and confirm no image pixels were changed.

**Pass gate:** Committed bounds are inside the canvas and have the requested ratio after pixel snapping. KNOWN OPEN RISK: the current PR has an unresolved rounding counterexample; any 201×200 committed 1:1 selection is FAIL and blocks parity signoff. This candidate build is for validation, not a waiver of that finding.

**Retain:** Info-panel width/height alongside selection ants, zoom and scaling, exact drag coordinates where available, and a short recording through release.

## G12 — Liquify brush resize stays anchored and paints nothing (PR 7919)

Source: https://github.com/storytold/photocraft/pull/7919

**Fixture:** Open grid-512.png, enter Liquify, record brush size and current deformation/history. Show the mesh/preview if supported.

1. Use the Windows Alt-drag resize gesture exposed by the app (the current implementation uses Ctrl+Alt plus primary drag); compare the corresponding Photoshop gesture.
2. Record the pointer's press position. Resize horizontally/vertically, and inspect the anchor and brush outline while held.
3. Release and move normally: the outline must follow the pointer again. Cancel and reopen to ensure no hidden stroke was recorded.
4. Start an ordinary warp stroke, then add Ctrl+Alt while still held. Record whether the gesture unexpectedly turns into a resize; compare Photoshop.
5. Repeat at 100%, 150% and 200% display scale and after opening on a second monitor.

**Pass gate:** Resize changes only brush size; no first-frame warp/dab, no anchor drift, and normal tracking returns after release. One-frame release outline lag is a recorded known minor behavior; modifier changes mid-stroke remain an explicit comparison gate.

**Retain:** Recording that includes press/hold/release, tool sizes before/after, deformation comparison PNG and native alt-drag test.

## G13 — Group Layers ATN targets survive cross-application playback (PR 7968)

Source: https://github.com/storytold/photocraft/pull/7968

**Fixture:** Use a PSD with uniquely named layers A, B and C, distinct colors, no duplicate names. Prepare actions targeting A explicitly while C is selected, implicit current-layer grouping, and Group Without Layers.

1. Record/save the grouping action in Photoshop, load it in PhotoCraft, reset the PSD and play with the deliberately different active layer.
2. Save the equivalent PhotoCraft action as ATN and load/play it in Photoshop on a fresh source copy.
3. Inspect which layer entered the group, group membership/order, and Undo.
4. Repeat no explicit target, Background target, and Photoshop-authored index-reference controls. Do not infer arbitrary index/id preservation from named-target coverage.

**Pass gate:** Explicit named targets stay explicit; implicit targets stay implicit; no unrelated active layer is grouped. Group Without Layers remains empty. Action definition is not changed by playback.

**Retain:** Expanded Actions step and Layers panels in both applications, original and exported ATNs, reset PSD and group membership notes.

## G14 — Emboss ATN uses Photoshop event and values (PR 7959)

Source: https://github.com/storytold/photocraft/pull/7959

**Fixture:** Use grid-512.png converted to an editable pixel layer. In Photoshop record Emboss with Angle 37°, Height 4 and Amount 123%; save the action set to ATN.

1. Load the Photoshop ATN in PhotoCraft; expand the step and confirm angle/height/amount before playback.
2. Reset the source and play; compare against Photoshop using the same supported depth and color mode.
3. Record or construct the equivalent PhotoCraft action, export ATN and play it in Photoshop.
4. Repeat a second setting and an ordinary default control. Verify refusal for unsupported mask/channel targeting instead of silent replay on layer pixels.

**Pass gate:** The action loads as Emboss, retains the three integer parameters and executes on the intended pixels. The Photoshop-authored ATN is the external oracle missing from the original approval; pixel differences are separately measured, not hidden by descriptor self-round-trip success.

**Retain:** Both applications' expanded Actions steps, Emboss dialogs, ATNs, source and output PNGs and the native Embs mapping test.

## G15 — Seeded Crystallize ATN export refuses silent changes (PR 7755)

Source: https://github.com/storytold/photocraft/pull/7755

**Fixture:** Use a native action/Actions JSON containing Crystallize with a nonzero seed; prepare otherwise identical absent-seed and numeric-zero controls, plus supported steps before/after it. The seed is an automation setting and need not exist in Photoshop's dialog.

1. Load the nonzero-seed action in PhotoCraft; run it natively and save its output.
2. Attempt strict Save Actions as ATN from the GUI. Capture the refusal naming the unsupported step; confirm the action itself and previous destination file are unchanged.
3. Exercise the explicit skipUnsupported route only as a separate opt-in check. Record exactly which step indices were omitted and play the resulting ATN in Photoshop.
4. Save/play absent-seed and zero-seed controls in both apps. Repeat malformed seeds and mask/channel target controls in native tests.
5. Check overlap with main's broader unholdable-setting checks; report duplicate guards separately without removing regression coverage.

**Pass gate:** A nonzero seed cannot silently export as zero. Strict failure is atomic; opt-in skipping is explicit and reported; absent/zero controls export normally. A screenshot of an error alone does not prove action/destination preservation.

**Retain:** Save Actions error screenshot, before/after action listing and destination hashes, skip report, ATNs and 8/16/32-bit native seeded tests.

## 4. Cross-feature and native desktop matrix

Run the core interaction cases at Windows scale 100%,150%,200%, on the primary monitor and after dragging the window to a second monitor with different scaling when available. Exercise the normal GPU backend; separately run the documented CPU canvas fallback. A software adapter is not physical-GPU validation. Record unavailable configurations as NOT_EXERCISED.

A. Open the warped/placed Smart Object PSD, Replace Contents, apply a filter, save/reopen, switch comps and return. Check pixels, selected layer, mask, dirty state, Undo/Redo and saved document.
B. Open the no-text image while a text document is loading; switch tabs, cancel the text open and save the raster document. No cross-document edits or orphan progress state.
C. Use Pencil and marquee in the same document, undo/reapply, then create slices and Delete a slice. Confirm no raster layer/pixels disappear.
D. Load Group Layers and Emboss actions, play on fresh source copies with different active layers, export/reload, and then test seeded Crystallize refusal. Keep native JSON action persistence separate from ATN compatibility.
E. Save cancellation, overwrite confirmation, unsupported/readonly destination and close/reopen must not corrupt an existing file. Use fixture copies, never simulate power loss on this graphical machine. PR 8034's durability change was excluded; ordinary save checks do not establish power-loss durability.
F. Launch from the extracted portable directory, verify the application icon, dialogs, taskbar/window, file association-free File > Open and drag/drop, and recovery after an ordinary forced application crash only in a disposable environment if authorized. Do not change Windows security policy to make the unsigned candidate launch.
G. Preserve Photoshop shortcut defaults as baseline guards: Preferences Ctrl+K, Crop C, Hide Layers Ctrl+comma where applicable. Verify existing Smart Object conversion/nesting behavior remains at baseline. These checks guard against accidental inclusion of held changes.
H. Repeat #6815 Light round-trip after an unrelated layer edit and #8250 forced re-render after #7857 replacement. Visual comparisons must not depend on stale cached composites.

## 5. Release decision

An artifact download or successful compilation means BUILT, not visually verified. The candidate may be accepted for the tested scope only after: exact-source native automated gates pass, every included PR is accounted for in ancestry and test inventory, every required graphical case has evidence, Photoshop cross-application files reopen, no unresolved regression remains, and the result owner records the scope and unexercised configurations.

G11's known fractional-edge issue must be resolved, excluded or explicitly left as a failing parity gate; do not call this build Photoshop-parity verified while it reproduces. G05's missing Light render is an acknowledged unsupported capability, not an unnoticed successful comparison. G06 requires wire-test evidence beyond graphics. G08 requires a real scan or a separate native lock-regression result.

Use results-template.json for attempts and final verdicts. Populate actual observations; no results are prefilled as PASS. Link every image/output and hash the completed evidence directory. Retain build source/ZIP identity and Photoshop System Info with the final report. Upstream PR approvals do not substitute for integration validation.

