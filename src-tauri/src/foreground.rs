use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};
use windows::core::PWSTR;

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

/// Detect the foreground application using the Win32 API chain:
/// GetForegroundWindow -> GetWindowThreadProcessId -> OpenProcess -> QueryFullProcessImageNameW.
///
/// UWP apps (ApplicationFrameHost.exe) are resolved to their real child process.
/// Returns DetectedApp with None fields if detection fails at any step.
pub fn detect_foreground_app() -> DetectedApp {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == std::ptr::null_mut() {
            return DetectedApp {
                exe_name: None,
                window_title: None,
            };
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return DetectedApp {
                exe_name: None,
                window_title: None,
            };
        }

        let window_title = get_window_title(hwnd);
        let mut exe_name = get_process_exe_name(pid);

        // UWP apps run inside ApplicationFrameHost.exe — resolve to real child process
        if let Some(ref name) = exe_name {
            if name == "applicationframehost.exe" {
                if let Some(child_name) = resolve_uwp_child(hwnd) {
                    exe_name = Some(child_name);
                }
            }
        }

        DetectedApp {
            exe_name,
            window_title,
        }
    }
}

/// Get the exe name for a process by PID.
///
/// Uses PROCESS_QUERY_LIMITED_INFORMATION to handle elevated processes gracefully —
/// returns None instead of crashing if access is denied.
/// CRITICAL: CloseHandle is called explicitly before any return path after OpenProcess.
unsafe fn get_process_exe_name(pid: u32) -> Option<String> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

    let mut buf = [0u16; 260]; // MAX_PATH
    let mut size = buf.len() as u32;

    let result = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_FORMAT(0),
        PWSTR(buf.as_mut_ptr()),
        &mut size,
    );

    // CRITICAL: Close handle before any return
    let _ = CloseHandle(handle);

    if result.is_err() {
        eprintln!("QueryFullProcessImageNameW failed for pid {}", pid);
        return None;
    }

    let full_path = String::from_utf16_lossy(&buf[..size as usize]);
    let filename = std::path::Path::new(&full_path)
        .file_name()?
        .to_string_lossy()
        .to_lowercase();

    Some(filename)
}

/// Get the window title text for a given HWND.
unsafe fn get_window_title(hwnd: HWND) -> Option<String> {
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buf);
    if len > 0 {
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    } else {
        None
    }
}

/// Resolve a UWP app's real child process from ApplicationFrameHost.exe.
///
/// Enumerates child windows of the parent HWND and returns the exe name of the
/// first child whose process is NOT applicationframehost.exe.
fn resolve_uwp_child(parent_hwnd: HWND) -> Option<String> {
    let mut result: Option<String> = None;

    unsafe {
        let _ = EnumChildWindows(
            parent_hwnd,
            Some(enum_child_proc),
            LPARAM(&mut result as *mut Option<String> as isize),
        );
    }

    result
}

/// Callback for EnumChildWindows. Finds the first child window whose process
/// is not applicationframehost.exe and stores its exe name via LPARAM.
///
/// Returns BOOL(0) to stop enumeration once a match is found, BOOL(1) to continue.
unsafe extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    if pid == 0 {
        return BOOL(1); // Continue enumeration
    }

    if let Some(name) = get_process_exe_name(pid) {
        if name != "applicationframehost.exe" {
            let result_ptr = lparam.0 as *mut Option<String>;
            *result_ptr = Some(name);
            return BOOL(0); // Stop enumeration — found real app
        }
    }

    BOOL(1) // Continue enumeration
}

/// Resolve whether ALL CAPS should be applied, considering per-app overrides.
///
/// Resolution order:
/// 1. If exe_name is Some and has a matching rule with all_caps = Some(v), return v
/// 2. Otherwise, fall back to profile_all_caps
pub fn resolve_all_caps(
    profile_all_caps: bool,
    exe_name: &Option<String>,
    rules: &HashMap<String, AppRule>,
) -> bool {
    if let Some(name) = exe_name {
        if let Some(rule) = rules.get(name) {
            if let Some(override_val) = rule.all_caps {
                return override_val;
            }
        }
    }
    profile_all_caps
}

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

    mod override_tests {
        use super::*;

        #[test]
        fn no_rule_profile_on() {
            let rules = HashMap::new();
            assert!(resolve_all_caps(true, &Some("notepad.exe".to_string()), &rules));
        }

        #[test]
        fn no_rule_profile_off() {
            let rules = HashMap::new();
            assert!(!resolve_all_caps(false, &Some("notepad.exe".to_string()), &rules));
        }

        #[test]
        fn force_on_overrides_profile_off() {
            let mut rules = HashMap::new();
            rules.insert("notepad.exe".to_string(), AppRule { all_caps: Some(true) });
            assert!(resolve_all_caps(false, &Some("notepad.exe".to_string()), &rules));
        }

        #[test]
        fn force_off_overrides_profile_on() {
            let mut rules = HashMap::new();
            rules.insert("notepad.exe".to_string(), AppRule { all_caps: Some(false) });
            assert!(!resolve_all_caps(true, &Some("notepad.exe".to_string()), &rules));
        }

        #[test]
        fn inherit_uses_profile_on() {
            let mut rules = HashMap::new();
            rules.insert("notepad.exe".to_string(), AppRule { all_caps: None });
            assert!(resolve_all_caps(true, &Some("notepad.exe".to_string()), &rules));
        }

        #[test]
        fn inherit_uses_profile_off() {
            let mut rules = HashMap::new();
            rules.insert("notepad.exe".to_string(), AppRule { all_caps: None });
            assert!(!resolve_all_caps(false, &Some("notepad.exe".to_string()), &rules));
        }

        #[test]
        fn detection_failed_falls_back_to_profile() {
            let mut rules = HashMap::new();
            rules.insert("notepad.exe".to_string(), AppRule { all_caps: Some(true) });
            assert!(!resolve_all_caps(false, &None, &rules));
        }

        #[test]
        fn unlisted_app_falls_back_to_profile() {
            let mut rules = HashMap::new();
            rules.insert("notepad.exe".to_string(), AppRule { all_caps: Some(true) });
            assert!(!resolve_all_caps(false, &Some("code.exe".to_string()), &rules));
        }
    }
}
