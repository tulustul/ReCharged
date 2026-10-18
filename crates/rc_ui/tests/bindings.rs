//! The committed TypeScript client matches the registered channels.

use std::path::Path;

#[test]
fn committed_typescript_bindings_are_current() {
    let dir = std::env::temp_dir().join(format!("rc_ui_bindings_{}", std::process::id()));
    let out = dir.join("bevy.ts");
    rc_ui::export_bindings(&out).expect("bindings export");
    let fresh = std::fs::read_to_string(&out).expect("exported bindings");
    let _ = std::fs::remove_dir_all(&dir);
    let committed_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui/src/bevy.ts");
    let committed = std::fs::read_to_string(committed_path).expect("ui/src/bevy.ts");
    assert!(
        fresh == committed,
        "ui/src/bevy.ts is stale: cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts"
    );
}
