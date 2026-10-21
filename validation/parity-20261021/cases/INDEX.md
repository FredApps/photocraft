# Full-batch Windows and Photoshop verification matrix

Every included upstream candidate gets a separate acceptance record. This is NOT_RUN; source integration, native automated tests and actual graphical evidence are separate gates. The curated core protocol supplies more detailed recipes for the initial file/tool regressions.

| PR | Target | Lane | Case |
|---|---|---|---|
| 8250 | fix(io): read a placed layer's custom warp over its source rect (#8227) | TYPE_VECTOR | [PR-8250](PR-8250.md) |
| 8114 | Layers panel: a click on the empty space below the layers deselects them (#8100) | INTERACTION | [PR-8114](PR-8114.md) |
| 8098 | PSD layer comps: an entry without enab inherits the previous visibility, as in Photoshop (#8093) | FILE_FIDELITY | [PR-8098](PR-8098.md) |
| 8097 | i18n(pt-br): translate 131 runtime labels that still showed in English | LANGUAGE | [PR-8097](PR-8097.md) |
| 8096 | CodeRabbit follow-ups for the #8095 batch | INTERACTION | [PR-8096](PR-8096.md) |
| 8074 | Transform: remember selected interpolation (#8067) | INTERACTION | [PR-8074](PR-8074.md) |
| 8053 | Pencil: align 1 px click snapping with the line walker (#4895) | INTERACTION | [PR-8053](PR-8053.md) |
| 8042 | Codecs: Fix corrupted CMYK JPEG exports (#8008) | FILE_FIDELITY | [PR-8042](PR-8042.md) |
| 8039 | Compose: Extend Softer outer glows past Size (#7899) | RENDERING | [PR-8039](PR-8039.md) |
| 8035 | CLI: reject ignored or conflicting mcp/serve launch options | AUTOMATION | [PR-8035](PR-8035.md) |
| 8023 | Keep '#' inside quoted .cube TITLE when parsing | FILE_FIDELITY | [PR-8023](PR-8023.md) |
| 8021 | Keep previous crop length when normalizing rounds to zero | INTERACTION | [PR-8021](PR-8021.md) |
| 8009 | automation: edit.installedProfiles is no host-path command | TYPE_VECTOR | [PR-8009](PR-8009.md) |
| 8005 | Open: a file without type never waits for the font scan (#7332) | TYPE_VECTOR | [PR-8005](PR-8005.md) |
| 8004 | Windows on ARM: auto picks DirectX 12 for Qualcomm graphics (#7537) | PLATFORM | [PR-8004](PR-8004.md) |
| 7994 | Fix Paint Bucket target with Gradient-style crosshair | INTERACTION | [PR-7994](PR-7994.md) |
| 7980 | perf(raster): tile content bounds compare rows in blocks (P32, #211) | PERFORMANCE | [PR-7980](PR-7980.md) |
| 7976 | Decode numeric character references in File Info XMP text | FILE_FIDELITY | [PR-7976](PR-7976.md) |
| 7968 | fix(atn): a Group Layers step keeps its explicit layer through .atn (#7236) | ACTIONS | [PR-7968](PR-7968.md) |
| 7966 | Slice Select gets its own icon: the Slice knife with a pointer (#7859) | INTERACTION | [PR-7966](PR-7966.md) |
| 7963 | Fix File Info showing 1/1 s for exposures just under one second | FILE_FIDELITY | [PR-7963](PR-7963.md) |
| 7959 | Actions: Emboss saves to and loads from .atn as Photoshop's Embs step (#7946) | ACTIONS | [PR-7959](PR-7959.md) |
| 7951 | Note tool: a selected note shows its author and colour in the options bar (#7940) | INTERACTION | [PR-7951](PR-7951.md) |
| 7947 | Type: hovering a served font fetches it after a short rest, not as the pointer passes | TYPE_VECTOR | [PR-7947](PR-7947.md) |
| 7932 | Layers: the pointer over a thumbnail shows what ⌘-click loads, with the +/−/× badge (#7824) | INTERACTION | [PR-7932](PR-7932.md) |
| 7928 | Fixed Size marquee: Width and Height in any length unit (#7910) | INTERACTION | [PR-7928](PR-7928.md) |
| 7924 | Smart Blur: 3.5× faster at radius 12 with identical output (#7921) | PERFORMANCE | [PR-7924](PR-7924.md) |
| 7922 | Create Rectangle and the other shape dialogs: right-click Width or Height to pick their unit (#7786) | TYPE_VECTOR | [PR-7922](PR-7922.md) |
| 7919 | Liquify: Alt-drag brush resize keeps the brush in place and no longer paints a dab (#7884) | INTERACTION | [PR-7919](PR-7919.md) |
| 7894 | Slice Select: Delete deletes the selected slice, not the layer (#7862) | INTERACTION | [PR-7894](PR-7894.md) |
| 7870 | Shortcuts: Ctrl+F4 closes the document beside Ctrl+W (Windows/Linux) | PLATFORM | [PR-7870](PR-7870.md) |
| 7857 | Replace Contents: contents of another size keep the placement's centre, as in Photoshop (#7844) | INTERACTION | [PR-7857](PR-7857.md) |
| 7853 | Brush: with airbrush build-up on, number keys set Flow and Shift+number sets Opacity (#7785) | INTERACTION | [PR-7853](PR-7853.md) |
| 7846 | Scale layer-effect noise by coverage so it follows the glow | RENDERING | [PR-7846](PR-7846.md) |
| 7839 | Copy: text selected in an error notice is copied, not the layer (#7807) | TYPE_VECTOR | [PR-7839](PR-7839.md) |
| 7838 | Free Transform: a result may take what Memory Usage allows, not 2^30 px (#7807) | INTERACTION | [PR-7838](PR-7838.md) |
| 7837 | Free Transform: a result over the pixel limit says so, not "not enough memory" (#7807) | INTERACTION | [PR-7837](PR-7837.md) |
| 7821 | feat(ui): search font input field sticky on scroll (#7792) | TYPE_VECTOR | [PR-7821](PR-7821.md) |
| 7791 | Camera raw: a developed raw keeps its EXIF (RW2 included), so File Info shows its Camera Data (#6957) | FILE_FIDELITY | [PR-7791](PR-7791.md) |
| 7784 | Actions: a background filter's step is recorded only by the recording it started in (#7289) | ACTIONS | [PR-7784](PR-7784.md) |
| 7772 | Vanishing Point: perspective clone samples without allocating per pixel (#7491) | PERFORMANCE | [PR-7772](PR-7772.md) |
| 7769 | Lasso: a short straight segment places a vertex instead of closing the outline | INTERACTION | [PR-7769](PR-7769.md) |
| 7759 | Actions: a setting Photoshop's event cannot hold refuses the step | ACTIONS | [PR-7759](PR-7759.md) |
| 7755 | fix(actions): refuse lossy seeded Crystallize ATN export | ACTIONS | [PR-7755](PR-7755.md) |
| 7751 | Actions: a Selective Color step keeps the values of its scalar form | ACTIONS | [PR-7751](PR-7751.md) |
| 7743 | packaging: the portable zip holds the files, not a folder with the files in it | INTERACTION | [PR-7743](PR-7743.md) |
| 7742 | Actions: a fractional setting an integer field cannot hold is refused, not rounded | ACTIONS | [PR-7742](PR-7742.md) |
| 7721 | fix(actions): refuse lossy fractional Crystallize ATN export | ACTIONS | [PR-7721](PR-7721.md) |
| 7708 | fix(actions): refuse lossy fractional Ripple ATN export | ACTIONS | [PR-7708](PR-7708.md) |
| 7696 | fix(actions): preserve effective Stroke width in ATN export | ACTIONS | [PR-7696](PR-7696.md) |
| 7681 | fix(actions): preserve scalar Selective Color settings in ATN export | ACTIONS | [PR-7681](PR-7681.md) |
| 7661 | Move tool: a selection lifts up to a pixel's own coverage (#7655) | RENDERING | [PR-7661](PR-7661.md) |
| 7659 | Transform a type layer with a missing font as its pixels (#7278) | TYPE_VECTOR | [PR-7659](PR-7659.md) |
| 7656 | Refuse .atn export of resolution-only Image Size without keepPrintSize | ACTIONS | [PR-7656](PR-7656.md) |
| 7653 | Move typed text with a drag from just outside it (#5956) | TYPE_VECTOR | [PR-7653](PR-7653.md) |
| 7647 | Carry on an open path from its end with the Pen (#6593) | TYPE_VECTOR | [PR-7647](PR-7647.md) |
| 7640 | Open the selection menu on a Quick Selection right-click (#7268) | INTERACTION | [PR-7640](PR-7640.md) |
| 7636 | Show Technique and Range in the glow Layer Style pages (#6698) | RENDERING | [PR-7636](PR-7636.md) |
| 7627 | Actions: Minimum / Maximum with Preserve: Roundness is no longer exported as Squareness (#7233) | ACTIONS | [PR-7627](PR-7627.md) |
| 7626 | Flatpak: a second launch hands its files to the running PhotoCraft over a Unix socket (#1540) | PLATFORM | [PR-7626](PR-7626.md) |
| 7622 | Show every blend mode in the open dropdown list (#7261) | RENDERING | [PR-7622](PR-7622.md) |
| 7617 | Studio: the floating Properties card has the Adjustments tab (#7265) | INTERACTION | [PR-7617](PR-7617.md) |
| 7611 | Refuse strict ATN export of Select and Mask with brush strokes | ACTIONS | [PR-7611](PR-7611.md) |
| 7599 | feat(engine): parameter schema types and display-string renderer | STRUCTURAL | [PR-7599](PR-7599.md) |
| 7595 | Toolbar: colour chips stay centred when the tools scroll (#7316) | INTERACTION | [PR-7595](PR-7595.md) |
| 7593 | Refuse .atn export of crops with output size or resolution | ACTIONS | [PR-7593](PR-7593.md) |
| 7591 | fix(atn): refuse strict export of canvas-bound selection modifies and placed pattern buckets | ACTIONS | [PR-7591](PR-7591.md) |
| 7590 | fix(atn): keep a Transform Selection's interpolation; refuse what the transform descriptor can't carry | ACTIONS | [PR-7590](PR-7590.md) |
| 7588 | fix(atn): refuse strict export of options the descriptors can't carry | ACTIONS | [PR-7588](PR-7588.md) |
| 7587 | Actions: stop when a Save step's document is closed by a Save Document script (#7367) | ACTIONS | [PR-7587](PR-7587.md) |
| 7585 | Refuse strict .atn export of a mask/channel-targeted Free Transform | ACTIONS | [PR-7585](PR-7585.md) |
| 7583 | Record actions.conditional as one step with its predicate | ACTIONS | [PR-7583](PR-7583.md) |
| 7580 | Preserve saved PCraft previews when manifest replacement fails | FILE_FIDELITY | [PR-7580](PR-7580.md) |
| 7576 | Presets: an Export As preset survives a damaged sibling set (test for #6906) | FILE_FIDELITY | [PR-7576](PR-7576.md) |
| 7572 | Menu bar: Alt held through a shortcut no longer takes the keyboard, so Enter reaches the dialog (#7481) | INTERACTION | [PR-7572](PR-7572.md) |
| 7571 | Refuse strict .atn export of Apply Image with a blending mask | ACTIONS | [PR-7571](PR-7571.md) |
| 7433 | Type: add the Warp Text button to the options bar | TYPE_VECTOR | [PR-7433](PR-7433.md) |
| 7397 | Auto-Blend Layers: turn the Background into a layer before masking it | RENDERING | [PR-7397](PR-7397.md) |
| 7396 | Auto-Blend Layers: Seamless Tones and Colors turns on gain compensation | RENDERING | [PR-7396](PR-7396.md) |
| 7395 | Auto-Blend Layers: mask and blend the whole layers, past the canvas too | RENDERING | [PR-7395](PR-7395.md) |
| 7389 | fix(atn): a preset-only Photo Filter step exports its resolved colour (#7235) | ACTIONS | [PR-7389](PR-7389.md) |
| 7385 | Content-Aware Fill workspace: an erased sampling area previews as an error, and a failed OK keeps it open (#7384) | INTERACTION | [PR-7385](PR-7385.md) |
| 7353 | test(engine): make the define-brush mask allocation fail on its own (fail-after-one) (#212) | STRUCTURAL | [PR-7353](PR-7353.md) |
| 7317 | Color panel: RGB slider mode (#7172) | INTERACTION | [PR-7317](PR-7317.md) |
| 7305 | Codecs: read Photoshop Raw (.raw) with retained header | FILE_FIDELITY | [PR-7305](PR-7305.md) |
| 7304 | Freeform Pen: ⇧ constrains freehand strokes to straight 45° lines | TYPE_VECTOR | [PR-7304](PR-7304.md) |
| 7303 | Keyboard Shortcuts: Shift is recorded when capturing a shortcut | INTERACTION | [PR-7303](PR-7303.md) |
| 7271 | macOS: keep the bundle's Dock icon after the window opens | PLATFORM | [PR-7271](PR-7271.md) |
| 7270 | Brushes: folders with search matches can be collapsed while searching | INTERACTION | [PR-7270](PR-7270.md) |
| 7262 | fix(atn): reject numeric layer references in .atn export (#7248) | ACTIONS | [PR-7262](PR-7262.md) |
| 7260 | test(io): SVG text-transform test can't pass without a font (#4806) | STRUCTURAL | [PR-7260](PR-7260.md) |
| 7258 | fix(atn): reject off-center radial blur in .atn export (#7234) | ACTIONS | [PR-7258](PR-7258.md) |
| 7249 | fix(atn): reject non-replace select.rect modes in .atn export (#7194) | ACTIONS | [PR-7249](PR-7249.md) |
| 7237 | Camera Raw: vignette Midpoint 0 darkens only the corners | FILE_FIDELITY | [PR-7237](PR-7237.md) |
| 7232 | fix(atn): export recorded layer.select with name parameter (#7185) | ACTIONS | [PR-7232](PR-7232.md) |
| 7227 | Liquify: zoom the preview with the zoom keys, a trackpad pinch and ×1.1 wheel notches (#7210) | INTERACTION | [PR-7227](PR-7227.md) |
| 7204 | Layer effects in Dissolve mode speckle, as in Photoshop (#6995) | RENDERING | [PR-7204](PR-7204.md) |
| 7167 | fix(ui): right-click an Adaptive Wide Angle constraint for its orientation menu, as in Photoshop | INTERACTION | [PR-7167](PR-7167.md) |
| 7161 | i18n(zh-hans): translate the 14 counted status messages missing from Simplified Chinese | LANGUAGE | [PR-7161](PR-7161.md) |
| 7160 | i18n(ko): translate the 14 counted status messages missing from Korean | LANGUAGE | [PR-7160](PR-7160.md) |
| 7159 | i18n(id): translate the 14 counted status messages missing from Indonesian | LANGUAGE | [PR-7159](PR-7159.md) |
| 7153 | Smart Filters: double-clicking a Liquify filter reopens it in the Liquify workspace | RENDERING | [PR-7153](PR-7153.md) |
| 7152 | Type: register craft fonts of every script, not only the Japanese ones | TYPE_VECTOR | [PR-7152](PR-7152.md) |
| 7150 | i18n(pl): translate the 14 counted status messages missing from Polish | LANGUAGE | [PR-7150](PR-7150.md) |
| 7142 | perf(type): refresh type layers without cloning the document (#7092) | PERFORMANCE | [PR-7142](PR-7142.md) |
| 7109 | Presets: the damaged-set test also covers Export As presets | FILE_FIDELITY | [PR-7109](PR-7109.md) |
| 7072 | Layer Style: keep gradient overlay colours when reopening the dialog (#7031) | INTERACTION | [PR-7072](PR-7072.md) |
| 7058 | Crop: X with a ratio preset turns the frame instead of shrinking it (#7057) | INTERACTION | [PR-7058](PR-7058.md) |
| 7021 | Eyedropper: a large sampling ring that reads on any colour (#6973) | INTERACTION | [PR-7021](PR-7021.md) |
| 6983 | fix(engine): File Info exposure keeps its 1/n form (#6958) | FILE_FIDELITY | [PR-6983](PR-6983.md) |
| 6942 | Color Picker: update the colour on press, not on release (#6937) | INTERACTION | [PR-6942](PR-6942.md) |
| 6919 | Selections don't mark a document modified (#6865) | INTERACTION | [PR-6919](PR-6919.md) |
| 6918 | fix(shortcuts): Page Up/Down and arrow-key shortcuts never fired (#6827) | INTERACTION | [PR-6918](PR-6918.md) |
| 6912 | Shortcuts: custom bindings on PageUp/PageDown (and other multi-word keys) fire again (#6827) | INTERACTION | [PR-6912](PR-6912.md) |
| 6910 | Web: one press per tap, so menus open again on touch (#2095) | INTERACTION | [PR-6910](PR-6910.md) |
| 6908 | Background redraw: keep the last finished composites, so a solo's restore uploads instead of recompositing (part of #6528) | FILE_FIDELITY | [PR-6908](PR-6908.md) |
| 6907 | test(ui): a toolbar click with no prior hover picks the tool (part of #2603) | STRUCTURAL | [PR-6907](PR-6907.md) |
| 6890 | Remove missing files from Open Recent and Home (#6610) | INTERACTION | [PR-6890](PR-6890.md) |
| 6881 | Actions: .atn Select Subject, Select and Mask, Color Range, clipping and layer masks, colours map to commands (part of #6745) | ACTIONS | [PR-6881](PR-6881.md) |
| 6873 | HEIF: honour the nclx colour description; HLG/PQ HEICs open as linear BT.2020 32-bit (#6752) | FILE_FIDELITY | [PR-6873](PR-6873.md) |
| 6868 | Keep shape fill and stroke alpha through PSD round-trip (#6180) | FILE_FIDELITY | [PR-6868](PR-6868.md) |
| 6837 | Hue/Saturation: hue tracks and spectrum bars use full-strength sRGB hues, one shared hue_color (#6540) | INTERACTION | [PR-6837](PR-6837.md) |
| 6835 | Object Finder: hovering with an empty active layer shows the status-bar hint (#6549) | INTERACTION | [PR-6835](PR-6835.md) |
| 6833 | Title bar: only the primary button starts a window drag (#6729) | INTERACTION | [PR-6833](PR-6833.md) |
| 6830 | Tab in Full Screen Mode shows the menu bar and panels; leaving Full Screen restores them as before (#6760) | INTERACTION | [PR-6830](PR-6830.md) |
| 6829 | Actions: Shift-click layer select in .atn imports as range select and exports back (#6784) | ACTIONS | [PR-6829](PR-6829.md) |
| 6826 | Don't pan the canvas when a dialog's title bar is dragged (#6810) | INTERACTION | [PR-6826](PR-6826.md) |
| 6815 | PSD: keep Photoshop 2026 Light adjustment layers verbatim and warn (#6542) | FILE_FIDELITY | [PR-6815](PR-6815.md) |
| 6813 | A layer that doesn't fit in GPU memory no longer breaks GPU drawing: older pages are freed and the layer is retried | FILE_FIDELITY | [PR-6813](PR-6813.md) |
| 6801 | Actions that start with Deselect play on when nothing is selected | INTERACTION | [PR-6801](PR-6801.md) |
| 6794 | feat(ui): System Info shows the graphics card's own memory | INTERACTION | [PR-6794](PR-6794.md) |
| 6734 | Polygonal Lasso: the last corner of a small shape no longer closes it (#6578) | TYPE_VECTOR | [PR-6734](PR-6734.md) |
| 6651 | Add a curated pattern library: 38 CC0 photographic tiles | INTERACTION | [PR-6651](PR-6651.md) |
| 6629 | Canvas overlays stay visible while panning (#6592) | INTERACTION | [PR-6629](PR-6629.md) |
| 6510 | Automation: resolve configured root aliases for absolute paths (#6426) | FILE_FIDELITY | [PR-6510](PR-6510.md) |
| 6509 | Type: commit the edit when another layer or document becomes active (#6483) | TYPE_VECTOR | [PR-6509](PR-6509.md) |
| 6492 | Fail control startup promptly when its port is occupied | AUTOMATION | [PR-6492](PR-6492.md) |
| 6481 | perf(hdr): Merge to HDR Pro and HDR Toning 3× faster, bit for bit the same | PERFORMANCE | [PR-6481](PR-6481.md) |
| 6472 | fix(ui): preserve shape stroke settings at zero width | TYPE_VECTOR | [PR-6472](PR-6472.md) |
| 6468 | fix(photocraft): `--control 0` logs the port actually bound (#6427) | AUTOMATION | [PR-6468](PR-6468.md) |
| 6464 | Survive a lost window GPU device: draw nothing, save unsaved work for recovery, close (#3438, #1215) | FILE_FIDELITY | [PR-6464](PR-6464.md) |
| 6456 | perf(quantize): Indexed Color palette build 8× faster, bit for bit the same | PERFORMANCE | [PR-6456](PR-6456.md) |
| 6417 | Context menus: an overflowing menu shows ▲/▼ scroll arrows (#2091) | TYPE_VECTOR | [PR-6417](PR-6417.md) |
| 6415 | Save for Web: Width, Height, Percent and Resample in Image Size, linked in proportion (#6388) | FILE_FIDELITY | [PR-6415](PR-6415.md) |
| 6410 | Artboards: layers outside the boards stay visible over the pasteboard (#5712) | INTERACTION | [PR-6410](PR-6410.md) |
| 6402 | CI: ready for a merge queue: merge_group trigger and one required `ci passed` check (#6400) | INTERACTION | [PR-6402](PR-6402.md) |
| 6401 | PSD smart filters: Sharpen, Dust & Scratches and Hue/Saturation stay editable (#6267) | FILE_FIDELITY | [PR-6401](PR-6401.md) |
| 6387 | Video Frames to Layers: every page of a multi-page TIFF (part of #6379) | FILE_FIDELITY | [PR-6387](PR-6387.md) |
| 6383 | Windows releases ship a symbols zip: the release build's PDBs for profilers (#688) | INTERACTION | [PR-6383](PR-6383.md) |
| 6380 | Make brush cursors follow pen orientation and projection | INTERACTION | [PR-6380](PR-6380.md) |
| 6363 | New Document: Color Profile, the working space or another profile of the mode (#321) | INTERACTION | [PR-6363](PR-6363.md) |
| 6307 | fix(io): open DDS textures with their alpha as an Alpha 1 channel | TYPE_VECTOR | [PR-6307](PR-6307.md) |
| 6271 | EXR: stage flat data windows on the display canvas | FILE_FIDELITY | [PR-6271](PR-6271.md) |
| 6253 | fix(compose): interior effects see the backdrop through a Normal layer's fill | RENDERING | [PR-6253](PR-6253.md) |
| 6244 | Preserve opaque edges when placing scaled Smart Objects | INTERACTION | [PR-6244](PR-6244.md) |
| 6243 | GPU: pin the keep-overlap ordering with a tile-growth regression test | RENDERING | [PR-6243](PR-6243.md) |
| 6224 | Free Transform: a rotated sprite's soft edge keeps its colour instead of a light border | INTERACTION | [PR-6224](PR-6224.md) |
| 6200 | Make Crystallize 4x and Pointillize 3x faster on 24 MP layers | PERFORMANCE | [PR-6200](PR-6200.md) |
| 6146 | refactor: share hsb conversions between photocraft-color and the color picker | STRUCTURAL | [PR-6146](PR-6146.md) |
| 6128 | Brush Preset picker: the cards start at the compact scale (#6107) | INTERACTION | [PR-6128](PR-6128.md) |
| 6112 | fix(compose): stop effect noise from painting empty blur tails | RENDERING | [PR-6112](PR-6112.md) |
| 6083 | New Guide Layout: format in ruler unit, support arbitrary units, and normalize on lost focus (#6082) | INTERACTION | [PR-6083](PR-6083.md) |
| 6026 | Marquee: Fixed Size selects exactly the entered width and height (fixes #6015) | INTERACTION | [PR-6026](PR-6026.md) |
| 6025 | Rectangular Marquee: Style: Fixed Size selects exactly that many pixels (#6015) | INTERACTION | [PR-6025](PR-6025.md) |
| 5940 | History panel: scroll to the current state on document change (#5909) | INTERACTION | [PR-5940](PR-5940.md) |
| 5939 | fix(paint): scale blur sigma with brush size (#5892) | RENDERING | [PR-5939](PR-5939.md) |
| 5938 | fix(ui): allow clipboard operations in modal dialogs and color picker (#5904) | INTERACTION | [PR-5938](PR-5938.md) |
| 5937 | fix(compose): prevent outside stroke from bleeding into opaque character interior (#5861) | RENDERING | [PR-5937](PR-5937.md) |
| 5928 | BMP export: Indexed Color documents save as palette BMPs | FILE_FIDELITY | [PR-5928](PR-5928.md) |
| 5927 | Fix Exposure value-field steps when decimals are set (#5914) | INTERACTION | [PR-5927](PR-5927.md) |
| 5907 | Text: apply anti-alias mode shaping when rasterizing type (#5902) | TYPE_VECTOR | [PR-5907](PR-5907.md) |
| 5891 | Panels: Collapse to Icons and Expand Panels for the dock column (#4059) | INTERACTION | [PR-5891](PR-5891.md) |
| 5878 | Preferences: old settings files take Shift-to-switch-tools on (#5647) | INTERACTION | [PR-5878](PR-5878.md) |
| 5877 | fix(format): ignore accidental ZIP directory signatures in comments | INTERACTION | [PR-5877](PR-5877.md) |
| 5870 | fix(format): reject duplicate native layer identities | INTERACTION | [PR-5870](PR-5870.md) |
| 5865 | Save: a flat file that keeps the document is saved in place (#5733) | FILE_FIDELITY | [PR-5865](PR-5865.md) |
| 5855 | Displace: OK asks for the displacement map, as in Photoshop (#5806) | INTERACTION | [PR-5855](PR-5855.md) |
| 5851 | Magic Wand: click on already-selected pixels deselects in Replace mode (#5775) | INTERACTION | [PR-5851](PR-5851.md) |
| 5816 | Auto-Blend Layers: blend a layer the canvas crops to all-overlap (#5745) | RENDERING | [PR-5816](PR-5816.md) |
| 5729 | Move tool: ΔX/ΔY readout beside the pointer while dragging (#5727) | INTERACTION | [PR-5729](PR-5729.md) |
| 5631 | Workspace: list saved workspaces in Window › Workspace and the switcher (storytold/photocraft#5610) | FILE_FIDELITY | [PR-5631](PR-5631.md) |
| 5607 | PSD: PSB foreign-order long-key blocks keep 8-byte length in little-endian TIFFs (#5599) | FILE_FIDELITY | [PR-5607](PR-5607.md) |
| 5590 | Warp: apply a large layer's warp in the background, with progress and Esc to cancel (#1015) | TYPE_VECTOR | [PR-5590](PR-5590.md) |
| 5581 | Layers: Group Layers creates the group collapsed (#5571) | INTERACTION | [PR-5581](PR-5581.md) |
| 5577 | Make the Pencil cursor outline the pixels its dab fills | INTERACTION | [PR-5577](PR-5577.md) |
| 5545 | Keep Free Transform preview colors consistent with the canvas | INTERACTION | [PR-5545](PR-5545.md) |
| 5515 | Selecting subject now automatically applies to selected layer, and us… | INTERACTION | [PR-5515](PR-5515.md) |
| 5452 | UI: split shape options and gradient stop helpers into reusable pieces (no behaviour change) | STRUCTURAL | [PR-5452](PR-5452.md) |
| 5445 | Restore Ctrl/Cmd + mouse-wheel zoom (#5363) | INTERACTION | [PR-5445](PR-5445.md) |
| 5420 | fix(format): report a failed directory flush after a save instead of ignoring it | FILE_FIDELITY | [PR-5420](PR-5420.md) |
| 5388 | docs: make contributor handoffs visible in PR descriptions | STRUCTURAL | [PR-5388](PR-5388.md) |
| 5370 | fix(gpu): catch validation errors in watched compositor work | RENDERING | [PR-5370](PR-5370.md) |
| 5300 | --version: name the commit of a dev build from git (#5247) | AUTOMATION | [PR-5300](PR-5300.md) |
| 5169 | fix(ui): show Paint Bucket first in the shared toolbar slot | INTERACTION | [PR-5169](PR-5169.md) |
| 5064 | docs(perf): how to measure one scenario and read its numbers | STRUCTURAL | [PR-5064](PR-5064.md) |
| 4993 | Prevent control-channel engine commands from editing behind modal dialogs | INTERACTION | [PR-4993](PR-4993.md) |
| 4991 | Deselect layers when clicking the pasteboard with Move Auto-Select | INTERACTION | [PR-4991](PR-4991.md) |
| 4935 | PSD Duotone: read and write the ink block (names, colours, curves) | FILE_FIDELITY | [PR-4935](PR-4935.md) |
| 4762 | fix(screen-picker): cancel queued requests without stale replies | INTERACTION | [PR-4762](PR-4762.md) |
| 4759 | Record Desaturate, Equalize, and Replace Color as smart filters. | RENDERING | [PR-4759](PR-4759.md) |
| 4743 | fix(ui): disable toolbar and tool options on the home screen | INTERACTION | [PR-4743](PR-4743.md) |
| 4742 | fix(macos): bundle photocraft-cli and resolve it from droplet scripts | PLATFORM | [PR-4742](PR-4742.md) |
| 4585 | Prevent clipboard centering overflow and incorrect paste placement | INTERACTION | [PR-4585](PR-4585.md) |
| 4547 | fix(icons): PhotoCraft icon corner radius matches other Craft apps (#4478) | INTERACTION | [PR-4547](PR-4547.md) |
| 4546 | fix(ui): Move tool shows transform controls by default (#4402, #4497) | INTERACTION | [PR-4546](PR-4546.md) |
| 4305 | Filter > Other > Frequency Separation (#1714) | RENDERING | [PR-4305](PR-4305.md) |
| 4219 | SVG import: set text in an installed font when the named ones are missing | FILE_FIDELITY | [PR-4219](PR-4219.md) |
| 3938 | Security: pin and verify the Windows signing client (#2936) | STRUCTURAL | [PR-3938](PR-3938.md) |
| 3926 | feat(ui): Free Transform 3x3 reference point widget and projective homography pivot snapping | INTERACTION | [PR-3926](PR-3926.md) |
| 3922 | fix(ui): make floating Properties available in Studio and Adwaita themes | INTERACTION | [PR-3922](PR-3922.md) |
| 3913 | fix(automation): exit idle MCP stdio servers when clients go quiet | AUTOMATION | [PR-3913](PR-3913.md) |
| 3874 | fix(layer-style): support Blend If Alt-split handle feathering and throttle live preview | RENDERING | [PR-3874](PR-3874.md) |
| 3554 | fix(packaging): launch bundled CLI directly from AppImage | PLATFORM | [PR-3554](PR-3554.md) |
| 3532 | Move tool: show all six Align icons and an icon menu for Distribute (#3490) | INTERACTION | [PR-3532](PR-3532.md) |
| 3520 | fix(ui): restore square color chips in Studio Dark theme | INTERACTION | [PR-3520](PR-3520.md) |
| 3387 | feat(codecs): add lossless JPEG XL export for 8/16-bit images | FILE_FIDELITY | [PR-3387](PR-3387.md) |
| 2945 | Open GIMP XCF files: layers, groups, masks, blend modes, channels and selection | FILE_FIDELITY | [PR-2945](PR-2945.md) |
| 2811 | select.cryptomatte: click an object, select its coverage | FILE_FIDELITY | [PR-2811](PR-2811.md) |
| 2324 | feat(toolbar): select grouped tools using press-drag-release | INTERACTION | [PR-2324](PR-2324.md) |
| 2300 | Scroll the viewport while outlining with Polygonal Lasso | INTERACTION | [PR-2300](PR-2300.md) |
| 1991 | feat(actions): import and export reusable Actions JSON files | FILE_FIDELITY | [PR-1991](PR-1991.md) |
