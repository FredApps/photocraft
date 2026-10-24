use super::*;
use photocraft_automation::{AuthorizedWorkspace, Headless};
use serde_json::json;

fn source() -> Vec<u8> {
    struct Wire(Vec<u8>);
    impl Wire {
        fn word(&mut self, n: u32) {
            self.0.extend(n.to_be_bytes());
        }
        fn pointer(&mut self, n: usize) {
            self.0.extend((n as u64).to_be_bytes());
        }
        fn slot(&mut self) -> usize {
            let at = self.0.len();
            self.pointer(0);
            at
        }
        fn patch(&mut self, at: usize) {
            let end = self.0.len() as u64;
            self.0[at..at + 8].copy_from_slice(&end.to_be_bytes());
        }
    }
    // Independent structural BE records, with explicitly LE U16 RGBA samples.
    let mut w = Wire(b"gimp xcf v011\0".to_vec());
    for n in [1, 1, 0, 250, 17, 1] {
        w.word(n);
    }
    w.0.push(0);
    w.word(0);
    w.word(0);
    let layer = w.slot();
    w.pointer(0);
    w.pointer(0);
    w.patch(layer);
    for n in [1, 1, 1, 6] {
        w.word(n);
    }
    w.0.extend(b"Pixel\0");
    w.word(0);
    w.word(0);
    let hierarchy = w.slot();
    w.pointer(0);
    w.patch(hierarchy);
    for n in [1, 1, 8] {
        w.word(n);
    }
    let level = w.slot();
    w.pointer(0);
    w.patch(level);
    w.word(1);
    w.word(1);
    let tile = w.slot();
    w.pointer(0);
    w.patch(tile);
    for n in [0x1234u16, 0x3456, 0x789a, 0x4000] {
        w.0.extend(n.to_le_bytes());
    }
    w.0
}

fn invoke(args: &[&str]) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(args.iter().copied(), &mut out, &mut err);
    (code, String::from_utf8(out).unwrap(), String::from_utf8(err).unwrap())
}

#[test]
fn historical_choice_uses_upstream_clap_validation_for_all_import_commands() {
    use crate::args::{Command, parse};
    let cases = [
        vec!["convert", "source.xcf", "native.pcraft"],
        vec!["info", "source.xcf"],
        vec!["run", "source.xcf", "--out", "native.pcraft"],
        vec!["batch", "--actions", "actions.json", "--in", "inputs", "--out", "outputs"],
    ];
    for base in cases {
        for (name, expected) in
            [("little-endian", photocraft_io::HistoricalXcfSampleOrder::LittleEndian), ("big-endian", photocraft_io::HistoricalXcfSampleOrder::BigEndian)]
        {
            let mut argv: Vec<OsString> = base.iter().map(OsString::from).collect();
            argv.extend(["--historical-xcf-sample-order".into(), name.into()]);
            let parsed = parse(&argv).unwrap();
            let choice = match parsed.command {
                Command::Convert(a) => a.import.historical_xcf_sample_order,
                Command::Info(a) => a.import.historical_xcf_sample_order,
                Command::Run(a) => a.import.historical_xcf_sample_order,
                Command::Batch(a) => a.import.historical_xcf_sample_order,
                _ => panic!("unexpected command"),
            };
            assert_eq!(choice, Some(expected));
            argv.extend(["--historical-xcf-sample-order".into(), name.into()]);
            assert_eq!(parse(&argv).err().unwrap().exit_code(), 2);
        }
        let mut argv: Vec<OsString> = base.iter().map(OsString::from).collect();
        argv.extend(["--historical-xcf-sample-order".into(), "guess".into()]);
        assert_eq!(parse(&argv).err().unwrap().exit_code(), 2);
        argv.pop();
        assert_eq!(parse(&argv).err().unwrap().exit_code(), 2);
    }
}

#[test]
fn historical_choice_reaches_cli_conversion_and_capability_scoped_rpc() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("photocraft-xcf-order-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    std::fs::create_dir(&dir).unwrap();
    let input = dir.join("source.xcf");
    let output = dir.join("native.pcraft");
    let bytes = source();
    std::fs::write(&input, &bytes).unwrap();
    let source_path = input.to_str().unwrap();
    let dest = output.to_str().unwrap();
    let (code, _, error) = invoke(&["convert", source_path, dest]);
    assert_eq!(code, 1, "{error}");
    assert!(error.contains("byte order"), "{error}");
    assert!(!output.exists());
    let (code, _, error) = invoke(&["convert", source_path, dest, "--historical-xcf-sample-order", "little-endian"]);
    assert_eq!(code, 0, "{error}");
    let reopened = photocraft_format::load_path(&output).unwrap();
    let pixels = reopened.layers[0].surface().unwrap().read_region(reopened.bounds());
    for (actual, expected) in pixels.iter().zip([0x1234u16, 0x3456, 0x789a, 0x4000]) {
        assert!((*actual - f32::from(expected) / 65535.).abs() < 0.000001);
    }
    assert_eq!(std::fs::read(&input).unwrap(), bytes);
    let (code, _, error) = invoke(&["info", source_path, "--historical-xcf-sample-order", "guess"]);
    assert_eq!(code, 2);
    assert!(error.contains("little-endian"), "{error}");
    let mut h = Headless::with_workspace(AuthorizedWorkspace::new(Some(&dir), None).unwrap());
    h.handle("doc.new", json!({"width": 2, "height": 2})).unwrap();
    let previous = h.session.active().unwrap().doc.id;
    assert!(h.handle("doc.open", json!({"path": "source.xcf"})).is_err());
    assert_eq!(h.session.active().unwrap().doc.id, previous);
    for invalid in [json!("guess"), json!(7), json!({})] {
        assert!(h.handle("doc.open", json!({"path": "source.xcf", "historicalXcfSampleOrder": invalid})).is_err());
        assert_eq!(h.session.active().unwrap().doc.id, previous);
    }
    h.handle("doc.open", json!({"path": "source.xcf", "historicalXcfSampleOrder": "little-endian"})).unwrap();
    let doc = &h.session.active().unwrap().doc;
    assert_eq!(doc.layers[0].surface().unwrap().read_region(doc.bounds()), pixels);
    assert!(h.handle("doc.open", json!({"path": "../source.xcf", "historicalXcfSampleOrder": "little-endian"})).is_err());
    drop(h);
    assert_eq!(dir.parent(), Some(std::env::temp_dir().as_path()));
    std::fs::remove_dir_all(dir).unwrap();
}
