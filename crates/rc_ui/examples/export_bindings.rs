//! Writes the typed React client without building the game:
//! `cargo run -p rc_ui --example export_bindings -- ui/src/bevy.ts`

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ui/src/bevy.ts".into());
    if let Err(e) = rc_ui::export_bindings(&path) {
        eprintln!("cannot write {path}: {e}");
        std::process::exit(1);
    }
    println!("wrote React bindings to {path}");
}
