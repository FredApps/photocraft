"""Isolated diagnostic: run unchanged scorecard generator without unrelated xtask UI deps."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path.cwd().resolve()
BASE = '1995d4bd9488313b5cb96e59d1805c05db778eb4'
HASHES = json.loads((ROOT / 'diagnostics/scorecard-source-hashes.json').read_text())
for name, expected in HASHES.items():
    data = (ROOT / f'xtask/src/{name}.rs').read_bytes()
    assert hashlib.sha256(data).hexdigest() == expected, f'source changed: {name}'
commit = subprocess.check_output(['git', 'cat-file', '-p', 'HEAD'], text=True)
assert [line[7:] for line in commit.split('\n\n', 1)[0].splitlines() if line.startswith('parent ')] == [BASE]
subprocess.run(['git', 'fetch', '--no-tags', '--depth=1', 'origin', BASE], check=True)
allowed = {'.github/workflows/scorecard-generate.yml', 'diagnostics/scorecard-fast.py', 'diagnostics/scorecard-source-hashes.json'}
changes = set(subprocess.check_output(['git', 'diff', '--name-only', BASE, 'HEAD'], text=True).splitlines())
assert changes == allowed, f'unreviewed diagnostic input changes: {changes}'
before_lock = (ROOT / 'Cargo.lock').read_bytes()
lock = tomllib.loads(before_lock.decode())
packages = lock['package']
def version(name):
    found = [p['version'] for p in packages if p['name'] == name]
    assert len(found) == 1, (name, found)
    return found[0]
with tempfile.TemporaryDirectory(prefix='scorecard-driver-', dir=os.environ['RUNNER_TEMP']) as tmp:
    driver = Path(tmp)
    assert not driver.is_relative_to(ROOT)
    (driver / 'src').mkdir()
    for name in HASHES:
        if name != 'main':
            shutil.copyfile(ROOT / f'xtask/src/{name}.rs', driver / f'src/{name}.rs')
    original = (ROOT / 'xtask/src/main.rs').read_text()
    helpers = original[original.index('/// A cargo command in the workspace root'):original.index('/// Two egui revisions')]
    # Exact original helper bodies; root is explicitly the original checkout, not this temp crate.
    main = '\n'.join('mod '+n+';' for n in HASHES if n != 'main') + '\n'
    main += 'use std::path::PathBuf;\nuse std::process::Command;\n'
    main += 'pub fn root() -> PathBuf { PathBuf::from(env!("SCORECARD_ORIGINAL_ROOT")) }\n'
    main += helpers
    main += '''fn main() -> Result<(), String> {
    scorecard::run(&root(), &[])?;
    scorecard::run(&root(), &["--check"])
}
'''
    (driver / 'src/main.rs').write_text(main)
    manifest = '[package]\nname="scorecard-driver"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\n'
    manifest += f'serde = {{ version="={version("serde")}", features=["derive", "rc"] }}\n'
    manifest += f'serde_json = "={version("serde_json")}"\n'
    manifest += f'toml = "={version("toml")}"\n'
    manifest += f'chrono = {{ version="={version("chrono")}", default-features=false, features=["std"] }}\n'
    (driver / 'Cargo.toml').write_text(manifest)
    (driver / 'Cargo.lock').write_bytes(before_lock)
    env = dict(os.environ, SCORECARD_ORIGINAL_ROOT=str(ROOT), CARGO_TARGET_DIR=str(driver / 'target'))
    # Resolve/prune the copied lock, then verify EVERY dependency version/checksum against original.
    result = subprocess.check_output(['cargo', 'metadata', '--format-version', '1'], cwd=driver, env=env, text=True)
    metadata = json.loads(result)
    assert len(metadata['packages']) <= 70, 'unexpected broad dependency graph'
    assert all(not p['name'].startswith(('craft', 'egui', 'photocraft')) for p in metadata['packages'])
    original_ids = {(p['name'], p['version'], p.get('source')):p.get('checksum') for p in packages}
    for p in tomllib.loads((driver / 'Cargo.lock').read_text())['package']:
        if p['name'] == 'scorecard-driver':
            continue
        key = (p['name'], p['version'], p.get('source'))
        assert key in original_ids and p.get('checksum') == original_ids[key], f'dependency drift: {key}'
        assert p.get('source', '').startswith('registry+'), f'non-registry dependency: {key}'
    print('Verified lightweight graph:', len(metadata['packages']), 'packages', flush=True)
    subprocess.run(['cargo', 'run', '--locked', '--manifest-path', str(driver/'Cargo.toml')], cwd=driver, env=env, check=True)
assert (ROOT / 'Cargo.lock').read_bytes() == before_lock
changed = subprocess.check_output(['git','diff','--name-only'], text=True).splitlines()
assert changed == ['docs/scorecard.md'], changed
subprocess.run(['git','diff','--check'], check=True)
subprocess.run(['git','diff','--','docs/scorecard.md'], check=True)
