use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// The foreground application detected via Win32 API.
///
/// `exe_name` is the lowercased bare filename (e.g. "notepad.exe").
/// `window_title` is the title bar text of the foreground window.
#[derive(Clone, Serialize)]
pub struct DetectedApp {
    pub exe_name: Option<String>,
    pub window_title: Option<String>,
}

/// Per-app override rule. Uses Option<bool> for three-state logic:
/// - None = inherit from profile (no override)
/// - Some(true) = force ON
/// - Some(false) = force OFF
#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct AppRule {
    pub all_caps: Option<bool>,
}

/// Tauri managed state holding per-app override rules.
/// Keyed by lowercased exe name (e.g. "notepad.exe" -> AppRule).
pub struct AppRulesState(pub std::sync::Mutex<HashMap<String, AppRule>>);

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_app_rule_serde_some_true() {
        let rule = AppRule { all_caps: Some(true) };
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: AppRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, deserialized);
        assert!(json.contains("true"));
    }

    #[test]
    fn test_app_rule_serde_some_false() {
        let rule = AppRule { all_caps: Some(false) };
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: AppRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, deserialized);
        assert!(json.contains("false"));
    }

    #[test]
    fn test_app_rule_serde_none() {
        let rule = AppRule { all_caps: None };
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: AppRule = serde_json::from_str(&json).unwrap();
        assert_eq!(rule, deserialized);
        assert!(json.contains("null"));
    }

    #[test]
    fn test_app_rule_roundtrip() {
        let rules = vec![
            AppRule { all_caps: Some(true) },
            AppRule { all_caps: Some(false) },
            AppRule { all_caps: None },
        ];
        for rule in rules {
            let json = serde_json::to_string(&rule).unwrap();
            let back: AppRule = serde_json::from_str(&json).unwrap();
            assert_eq!(rule, back);
        }
    }

    #[test]
    fn test_app_rules_map_serde() {
        let mut map = HashMap::new();
        map.insert("notepad.exe".to_string(), AppRule { all_caps: Some(true) });
        map.insert("code.exe".to_string(), AppRule { all_caps: Some(false) });
        map.insert("firefox.exe".to_string(), AppRule { all_caps: None });

        let json = serde_json::to_string(&map).unwrap();
        let back: HashMap<String, AppRule> = serde_json::from_str(&json).unwrap();
        assert_eq!(map, back);
    }

    #[test]
    fn test_app_rules_empty_map_serde() {
        let map: HashMap<String, AppRule> = HashMap::new();
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, "{}");
        let back: HashMap<String, AppRule> = serde_json::from_str(&json).unwrap();
        assert_eq!(map, back);
    }

    #[test]
    fn test_detected_app_serialize() {
        let app = DetectedApp {
            exe_name: Some("notepad.exe".to_string()),
            window_title: Some("Untitled - Notepad".to_string()),
        };
        let json = serde_json::to_value(&app).unwrap();
        assert_eq!(json["exe_name"], "notepad.exe");
        assert_eq!(json["window_title"], "Untitled - Notepad");
    }

    #[test]
    fn test_detected_app_serialize_none() {
        let app = DetectedApp {
            exe_name: None,
            window_title: None,
        };
        let json = serde_json::to_value(&app).unwrap();
        assert!(json["exe_name"].is_null());
        assert!(json["window_title"].is_null());
    }
}
