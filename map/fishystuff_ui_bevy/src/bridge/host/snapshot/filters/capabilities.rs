pub(in crate::bridge::host::snapshot) fn bridge_capabilities() -> Vec<String> {
    vec![
        "theme".to_string(),
        "patch-filter".to_string(),
        "layer-visibility".to_string(),
        "layer-order".to_string(),
        "layer-opacity".to_string(),
        "fish-filter".to_string(),
        "view-mode".to_string(),
        "selection".to_string(),
        "hover".to_string(),
        "diagnostic".to_string(),
        "restore".to_string(),
    ]
}
