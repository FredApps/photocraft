//! #7668: strict ATN export must not silently reset Crystallize's native random seed.
use photocraft_engine::actions_cmds::Action;
use photocraft_engine::{Session, atn_cmds};
use photocraft_psd::{atn, descriptor::Value as D};
use serde_json::{Value, json};

const CRYSTALLIZE: &str = "filter.pixelate.crystallize";

/// Procedural colours expose seed changes without external images or Photoshop assets.
fn fixture(depth: u32) -> Session {
    let mut s = Session::new();
    s.execute("file.new", json!({"width": 24, "height": 18, "depth": depth})).unwrap();
    s.edit("procedural fixture", |doc, active| {
        let surface = doc.layer_mut(active.unwrap()).unwrap().surface_mut().unwrap();
        let format = surface.format();
        let bytes: Vec<_> =
            (0..432).flat_map(|i| photocraft_raster::from_rgba(&format, [(i % 11) as f32 / 10.0, (i % 7) as f32 / 6.0, (i % 5) as f32 / 4.0, 1.0])).collect();
        surface.write_region(photocraft_doc::Rect::from_xywh(0, 0, 24, 18), &bytes);
        Ok(())
    })
    .unwrap();
    s
}

/// Compare flattened output, so a parameter-only assertion cannot hide a changed edit.
fn pixels(s: &Session) -> Vec<[f32; 4]> {
    photocraft_compose::flatten(&s.active().unwrap().doc).px
}

/// Nonzero seeds stay valid in native actions, but cannot silently become seed zero in ATN.
#[test]
fn nonzero_seeds_remain_native_and_are_refused_by_strict_export() {
    let mut silently_exported = Vec::new();
    for depth in [8, 16, 32] {
        for cell_size in [5, 10] {
            for seed in [1_u32, 37, 255, 65537, 2147483648, u32::MAX] {
                let params = json!({"cellSize": cell_size, "seed": seed});
                let action = Action::new("Seeded Crystallize", vec![(CRYSTALLIZE.into(), params.clone())]);
                let mut direct = fixture(depth);
                direct.execute(CRYSTALLIZE, params).unwrap();
                let expected = pixels(&direct);
                let mut native = fixture(depth);
                native.actions.list.push(action.clone());
                native.execute("actions.play", json!({"action": "Seeded Crystallize"})).unwrap();
                assert_eq!(pixels(&native), expected);
                let mut zero = fixture(depth);
                zero.execute(CRYSTALLIZE, json!({"cellSize": cell_size, "seed": 0})).unwrap();
                if seed == 37 {
                    assert_ne!(pixels(&zero), expected, "fixture must expose seed37 vs0: {depth}/{cell_size}");
                    let changed_components =
                        pixels(&zero).iter().zip(&expected).flat_map(|(a, b)| a.iter().take(3).zip(b.iter().take(3))).filter(|(a, b)| a != b).count();
                    eprintln!("seed37 vs0: depth{depth}/cell{cell_size}, changed RGB components={changed_components}");
                }
                if let Ok((bytes, skipped)) = atn_cmds::export("Crystallize", &[&action], &[], false) {
                    assert!(skipped.is_empty());
                    let imported = atn_cmds::import(&bytes).unwrap();
                    assert!(imported.unsupported.is_empty() && imported.warnings.is_empty());
                    assert!(imported.actions[0].steps[0].1.get("seed").is_none());
                    let mut replay = fixture(depth);
                    replay.actions.list = imported.actions;
                    replay.execute("actions.play", json!({"action": "Seeded Crystallize"})).unwrap();
                    assert_eq!(pixels(&replay), pixels(&zero));
                    silently_exported.push(format!("depth{depth}/cell{cell_size}/seed{seed}: pixels changed={}", pixels(&replay) != expected));
                }
            }
        }
    }
    assert!(silently_exported.is_empty(), "strict export silently reset native seeds:\n{}", silently_exported.join("\n"));
}

/// Missing and numeric-zero seeds retain direct/native/binary-ATN playback and the known descriptor.
#[test]
fn zero_seed_and_defaults_preserve_playback() {
    for depth in [8, 16, 32] {
        for params in [
            Value::Null,
            json!({}),
            json!({"target": "pixels"}),
            json!({"cellSize": 5}),
            json!({"seed": 0}),
            json!({"seed": 0.0}),
            json!({"cellSize": 5, "seed": 0, "target": "pixels"}),
            json!({"cellSize": 3, "seed": 0}),
            json!({"cellSize": 300, "seed": 0}),
        ] {
            let action = Action::new("Default Crystallize", vec![(CRYSTALLIZE.into(), params.clone())]);
            let mut direct = fixture(depth);
            direct.execute(CRYSTALLIZE, params.clone()).unwrap();
            let mut native = fixture(depth);
            native.actions.list.push(action.clone());
            native.execute("actions.play", json!({"action": "Default Crystallize"})).unwrap();
            assert_eq!(pixels(&native), pixels(&direct));
            let (bytes, skipped) = atn_cmds::export("Crystallize", &[&action], &[], false).unwrap();
            assert!(skipped.is_empty());
            let file = atn::parse(&bytes).unwrap();
            let descriptor = file.actions[0].items[0].descriptor.as_ref().unwrap();
            // No speculative Photoshop random-seed key is added to Crst.
            assert_eq!(descriptor.items.len(), usize::from(params.get("cellSize").is_some()));
            if let Some(cell_size) = params.get("cellSize").and_then(Value::as_i64) {
                assert!(matches!(descriptor.get("ClSz"), Some(D::Integer(n)) if i64::from(*n) == cell_size));
            }
            let imported = atn_cmds::import(&bytes).unwrap();
            assert!(imported.unsupported.is_empty() && imported.warnings.is_empty());
            assert_eq!(imported.actions[0].steps[0].0, CRYSTALLIZE);
            let mut replay = fixture(depth);
            replay.actions.list = imported.actions;
            replay.execute("actions.play", json!({"action": "Default Crystallize"})).unwrap();
            assert_eq!(pixels(&replay), pixels(&direct), "depth{depth}: {params}");
        }
    }
}

/// Explicit skipping names the lost steps and retains supported command order and playback.
#[test]
fn skipping_seeded_steps_reports_exact_indices_and_keeps_supported_edits() {
    let steps = vec![
        (CRYSTALLIZE.into(), json!({"cellSize": 5, "seed": 0})),
        (CRYSTALLIZE.into(), json!({"cellSize": 5, "seed": 37})),
        ("image.adjustments.invert".into(), json!({})),
        (CRYSTALLIZE.into(), json!({"cellSize": 10, "seed": u32::MAX})),
        (CRYSTALLIZE.into(), json!({"cellSize": 10})),
    ];
    let action = Action::new("Mixed seeds", steps.clone());
    let error = atn_cmds::export("Crystallize", &[&action], &[], false).unwrap_err();
    assert!(error.contains("step 2 of `Mixed seeds`") && error.contains(CRYSTALLIZE) && error.contains("skipUnsupported"), "{error}");
    let (bytes, skipped) = atn_cmds::export("Crystallize", &[&action], &[], true).unwrap();
    assert_eq!(
        skipped,
        vec![
            atn_cmds::Skipped { action: "Mixed seeds".into(), step: 1, id: CRYSTALLIZE.into() },
            atn_cmds::Skipped { action: "Mixed seeds".into(), step: 3, id: CRYSTALLIZE.into() },
        ]
    );
    let imported = atn_cmds::import(&bytes).unwrap();
    assert!(imported.unsupported.is_empty() && imported.warnings.is_empty());
    assert_eq!(imported.actions[0].steps.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(), [CRYSTALLIZE, "image.adjustments.invert", CRYSTALLIZE]);
    for depth in [8, 16, 32] {
        let mut expected = fixture(depth);
        for index in [0, 2, 4] {
            expected.execute(&steps[index].0, steps[index].1.clone()).unwrap();
        }
        let mut replay = fixture(depth);
        replay.actions.list = imported.actions.clone();
        replay.execute("actions.play", json!({"action": "Mixed seeds"})).unwrap();
        assert_eq!(pixels(&replay), pixels(&expected));
    }
}

/// The guard rejects malformed seeds and preserves the existing pixel-target restriction.
#[test]
fn invalid_seeds_and_non_pixel_targets_are_refused() {
    let mut cases: Vec<Value> = [json!(null), json!("0"), json!(false), json!([]), json!({}), json!(-1), json!(0.25), json!(4294967296_u64), json!(1e100)]
        .into_iter()
        .map(|seed| json!({"cellSize": 5, "seed": seed}))
        .collect();
    cases.extend([json!("mask"), json!("quickMask"), json!("channel"), Value::Null].map(|target| json!({"cellSize": 5, "seed": 0, "target": target})));
    for params in cases {
        let action = Action::new("Unsupported seed", vec![(CRYSTALLIZE.into(), params.clone())]);
        assert!(atn_cmds::export("Crystallize", &[&action], &[], false).is_err(), "{params}");
        let (bytes, skipped) = atn_cmds::export("Crystallize", &[&action], &[], true).unwrap();
        assert_eq!(skipped, vec![atn_cmds::Skipped { action: "Unsupported seed".into(), step: 0, id: CRYSTALLIZE.into() }]);
        assert!(atn_cmds::import(&bytes).unwrap().actions[0].steps.is_empty());
    }
}

/// Save Actions surfaces strict refusal without changing actions, and opt-in reports its loss.
#[test]
fn save_actions_command_refuses_seed_loss_and_reports_opt_in_skip() {
    let mut s = Session::new();
    s.actions.list.push(Action::new("Seeded", vec![(CRYSTALLIZE.into(), json!({"cellSize": 5, "seed": 37}))]));
    let before = s.actions.list[0].steps.clone();
    let error = s.execute("actions.save", json!({})).unwrap_err().to_string();
    assert!(error.contains("step 1 of `Seeded`") && error.contains(CRYSTALLIZE) && error.contains("skipUnsupported"), "{error}");
    assert_eq!(s.actions.list[0].steps, before);
    let saved = s.execute("actions.save", json!({"skipUnsupported": true})).unwrap();
    assert_eq!(saved["steps"], 0);
    assert_eq!(saved["skipped"], json!([{"action": "Seeded", "step": 0, "id": CRYSTALLIZE}]));
    let bytes = photocraft_paint::tile::b64_decode(saved["data"].as_str().unwrap()).unwrap();
    assert!(atn_cmds::import(&bytes).unwrap().actions[0].steps.is_empty());
    assert_eq!(s.actions.list[0].steps, before);
}
