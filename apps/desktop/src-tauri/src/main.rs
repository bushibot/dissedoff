// Deliberately thin: the frontend (apps/desktop/dist/index.html, the same
// UI the server serves itself at `/`) talks directly to whatever Dissedoff
// server URL the user enters, over plain REST + WebSocket. No custom Tauri
// commands exist yet because nothing here needs native OS access — that
// changes once push-to-talk/overlay (a native helper process, per PLAN.md)
// is built.
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Dissedoff desktop app");
}
