use dnacalc_host_core::{DocumentSession, RichTreeSession, SkinProtocolSession};
use dnacalc_skin_ir::{
    ExtensionPlacementProjection, HostCapabilityProjection, RuntimeProfileProjection, SkinSnapshot,
};

#[tauri::command]
fn native_capabilities() -> HostCapabilityProjection {
    desktop_capabilities()
}

#[tauri::command]
fn native_snapshot() -> SkinSnapshot {
    desktop_session().snapshot()
}

fn desktop_capabilities() -> HostCapabilityProjection {
    HostCapabilityProjection::treecalc_references(
        RuntimeProfileProjection::WindowsDesktop,
        ExtensionPlacementProjection::Unavailable,
    )
}

fn desktop_session() -> SkinProtocolSession {
    SkinProtocolSession::new(
        "native-empty",
        RuntimeProfileProjection::WindowsDesktop,
        ExtensionPlacementProjection::Unavailable,
        DocumentSession::RichTree(RichTreeSession::new()),
    )
    .expect("valid native host placement")
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            native_capabilities,
            native_snapshot
        ])
        .run(tauri::generate_context!())
        .expect("run DNA TreeCalc desktop shell");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_honest_native_placement() {
        let capabilities = desktop_capabilities();
        assert_eq!(
            capabilities.runtime_profile,
            RuntimeProfileProjection::WindowsDesktop
        );
        assert_eq!(
            capabilities.extension_placement,
            ExtensionPlacementProjection::Unavailable
        );
        assert_eq!(capabilities.validate(), Ok(()));
        let snapshot = desktop_session().snapshot();
        assert_eq!(
            snapshot.host_capabilities.runtime_profile,
            RuntimeProfileProjection::WindowsDesktop
        );
        assert_eq!(snapshot.validate(), Ok(()));
    }
}
