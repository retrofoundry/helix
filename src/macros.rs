#[macro_export]
macro_rules! helix {
    () => {
        $crate::HELIX.lock().unwrap()
    };
}
