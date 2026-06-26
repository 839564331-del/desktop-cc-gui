pub(super) fn runtime_key(engine: &str, workspace_id: &str) -> String {
    format!("{engine}::{workspace_id}")
}

pub(super) fn normalize_engine(engine: &str) -> String {
    let normalized = engine.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        "codex".to_string()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_engine, runtime_key};

    #[test]
    fn normalize_engine_trims_lowercases_and_defaults_empty_to_codex() {
        assert_eq!(normalize_engine(" Codex "), "codex");
        assert_eq!(normalize_engine("CLAUDE"), "claude");
        assert_eq!(normalize_engine("   "), "codex");
    }

    #[test]
    fn runtime_key_preserves_engine_and_workspace_identity() {
        assert_eq!(runtime_key("codex", "ws-1"), "codex::ws-1");
    }
}
