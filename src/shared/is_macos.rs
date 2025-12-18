pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}
