# Combined-build conflict decisions

This is an integration record, not a test result. All 221 source heads remain in ancestry. Native CI and the Photoshop acceptance matrix must validate the combined behavior.

| Candidates | Resolution and required evidence |
|---|---|
| 7751 / 7681 | Retain the shared Selective Color command parser, including relative/method precedence, scalar overrides and validation. Retain the additional scalar-form round-trip test. |
| 7742 / 7708 / 7588 | Use one exact-integer descriptor kind for both Ripple amount and Crystallize cell size. Retain general integer range checks and fractional/default controls from both PRs. |
| 7627 / 7759 | Retain the shared unsupported-setting guard for Minimum/Maximum Roundness, seeded Crystallize and off-centre Radial Blur. Add the independent Minimum/Maximum round-trip/refusal test. |
| 7611 / 6881 | Preserve modern Select and Mask mappings while refusing unrepresentable Refine Edge Brush strokes in both legacy and modern events. Keep default/empty-stroke controls and sample-all-layers mapping. |
| 7588 / 7968 / 7262 | Preserve named Group Layers targets rather than reverting to the selected-layer-only descriptor. The numeric-target refusal remains appropriate: numeric document IDs cannot be persisted as Photoshop layer references. Propagate fallible layer references in group/alignment mappings. |
| 7262 / 7784 | Keep both pre-selection capture and recording ownership for asynchronous jobs. A later recording must not capture an earlier edit; recorded targets must use the selection that existed before dispatch. |
| 7109 / 7576 | Keep both independently named damaged-preset tests, including export settings and recent-document coverage. Do not discard assertions to make the combined fixture pass. |
| 6983 / 7963 | Retain the three-significant-digit fallback for shorter subsecond exposures and require reciprocal denominators of at least two. Values whose reciprocal rounds to one retain the two-decimal display: 0.996 becomes 1 s, while 0.99 remains 0.99 s. |
| 6912 / 6918 | Retain the broader canonical key-name table and aliases, adding IntlBackslash. Keep both keyboard normalization/UI regressions. |
| 6468 / 6492 | Preserve the pre-bound listener architecture and its authentication test. Use the actual bound address for port-zero diagnostics and retain the new connection test. |
| 6402 | Preserve the merge-queue trigger and final CI aggregate gate; merge platform-selection documentation including filesystem automation. The native Windows unit and integration legs are required for this candidate. |
| 5937 | Preserve the newer outside-stroke fill/coverage handling and Dissolve path; retain the additional tests from the overlapping stroke PR. |
| 5865 | Preserve source frame/page counts while keeping both TGA and DDS alpha-to-channel handling. |
| 5452 / 6472 | Extract the shape toolbar into its new module while carrying forward zero-width stroke state and gesture-scoped undo coalescing. Keep the shape/pen UI regressions. |
| 5420 / 7580 | Flush reused object directories before committing the manifest. Keep stale-preview cleanup after manifest commit so a failed save leaves the previous document's previews intact. Retain durability and rollback tests. |
| 3922 / 7617 | Preserve floating Properties/Adjustments tabs and drag/hide controls. Put the selected tab's body inside the bounded scrolling area. |
| 7433 / 6651 / 3532 | Union original-icon attribution exclusions and retain ambientCG tile attribution/license. |
| 3387 | Retain both JPEG XL and Photoshop RAW codec modules in the shared import list. |
| 2811 | Register both Cryptomatte and frequency-separation command specifications. |
| 6734 | Integrate the originally approved head `be98937e996e8a2f5be777d0deafd5186e82afc8`. Preserve stale-polygon cancellation and the separate last-point hand-jitter threshold while adopting the small-polygon first-point close reach. Later branch head `e36bfb030edb8780bb52d1e5849c171a0ca26296` is outside this approved snapshot. |

Independent function/test insertions were merged with syntax-aware three-way text handling. When both PRs changed a self-contained test function, both versions were retained with distinct names. Rust sources were parsed and formatted without compiling or executing candidate code on the operator machine. Compilation and native tests run on GitHub-hosted runners.

The hosted combined runs also required these adaptations:

- Count both new codecs in the fixed-size format registry (22 entries).
- Propagate the filter API's error result in Frequency Separation, use the cached surface's format in text-transform memory checks, and use the shared HSB conversion in adjustment and angle widgets.
- Update retained regression fixtures for per-tool Sample All Layers state, text-edit document ownership and fallible Smart Filter results without removing their behavioral assertions.
- Give the EXR part-selection fixture explicit 4 by 4 and 2 by 2 display canvases. Both retain the selected part's 4 by 4 data window and color-rank assertions; the smaller display additionally proves clipping.
- Price the missing-font transform refusal fixture against the new byte budget: an RGBA8 result of 1000002 by 10 pixels exceeds the 20 MiB test budget. Retain document, revision and history preservation assertions and require the current pre-allocation refusal message.
- Run this fork's nextest jobs with `--no-fail-fast` to collect failures across the complete combined test set. Failed assertions still fail CI.
- Adapt retained UI regression fixtures to the combined defaults: explicitly disable Move transform controls when exercising ordinary layer drags or hidden widgets; open a document before asserting enabled toolbar input and exact chip colors; scroll mask sliders into the bounded floating Properties body before native pointer gestures. Keep undo, live-preview, no-trail, no-prior-hover and chip-position assertions. Fixed Size marquee expectations now use the approved containing-pixel anchoring behavior at fractional presses, while still asserting exact entered dimensions for rectangle and ellipse clicks/drags.
- Give three HDR test helpers the same Linux x86-64 configuration as the recorded-libm-hash tests that consume them. The existing portable serial-reference/blur tests remain enabled; no Windows test is removed and no warning is suppressed. Linux recorded hashes do not establish Windows floating-point identity.

The six excluded candidates are 7318 (Photoshop keyboard shortcut divergence), 8012 (freeze interpolation shift), 8034 (insufficient Windows durability CI evidence), 7970 (unconditional native format version change), 3342 (editable type perspective behavior differing from Photoshop), and 8043 (fractional-edge 1:1 selection rounding can become 201 by 200). See the candidate review and integration plan for source evidence. These exclusions do not imply the baseline has already solved the underlying issues.

Hosted full-suite follow-up: restore best-effort directory flushing for removed stale previews after manifest publication. The regression asserts that failed manifest flushing preserves the old preview, successful cleanup flushes after the manifest, absent previews cause no redundant flush, and a post-commit cleanup flush failure still returns a successful, readable save. Reused tile/blob folders still flush before publication.
