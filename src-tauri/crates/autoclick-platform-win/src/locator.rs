use autoclick_domain::config::TargetProfile;
use serde::{Deserialize, Serialize};

use crate::{
    PlatformError, process,
    window::{WindowInfo, enumerate_windows, inspect_window},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocatorCandidate {
    pub window: WindowInfo,
    pub reliability: u8,
    pub reason: String,
}

pub fn locate_target_window(
    target: &TargetProfile,
) -> Result<Option<LocatorCandidate>, PlatformError> {
    if let Some(hwnd) = target.hwnd {
        if let Some(window) = inspect_window(hwnd as isize)? {
            return Ok(Some(LocatorCandidate {
                window,
                reliability: 100,
                reason: "hwnd 精确匹配".to_string(),
            }));
        }
    }

    let mut best: Option<LocatorCandidate> = None;
    for window in enumerate_windows()? {
        let (score, reason) = score_window(&window, target);
        if score == 0 {
            continue;
        }
        let candidate = LocatorCandidate {
            window,
            reliability: score,
            reason,
        };
        if best
            .as_ref()
            .map(|value| candidate.reliability > value.reliability)
            .unwrap_or(true)
        {
            best = Some(candidate);
        }
    }
    Ok(best)
}

fn score_window(window: &WindowInfo, target: &TargetProfile) -> (u8, String) {
    if let Some(hwnd) = target.hwnd {
        if window.hwnd == hwnd as isize {
            return (100, "hwnd 精确匹配".to_string());
        }
    }

    let mut score = 0u8;
    let mut reasons = Vec::new();

    if target.strategies.process_name {
        if let Some(process_name) = &target.process_name {
            let current_name = window
                .exe_path
                .as_deref()
                .and_then(process::process_name_from_path)
                .unwrap_or_default();
            if match_text(&current_name, process_name, target.allow_partial_match) {
                score += 40;
                reasons.push("进程名匹配");
            }
        }
    }

    if target.strategies.process_path {
        if let Some(process_path) = &target.process_path {
            if match_text(
                window.exe_path.as_deref().unwrap_or_default(),
                process_path,
                target.allow_partial_match,
            ) {
                score += 30;
                reasons.push("进程路径匹配");
            }
        }
    }

    if target.strategies.window_title {
        if let Some(title) = &target.title_contains {
            if match_text(&window.title, title, true) {
                score += 20;
                reasons.push("标题匹配");
            }
        }
    }

    if target.strategies.class_name {
        if let Some(class_name) = &target.class_name {
            if match_text(&window.class_name, class_name, target.allow_partial_match) {
                score += 15;
                reasons.push("类名匹配");
            }
        }
    }

    if score == 0 && target.strategies.fuzzy_match {
        if let Some(title) = &target.title_contains {
            if match_text(&window.title, title, true) {
                score += 10;
                reasons.push("模糊标题匹配");
            }
        }
    }

    if score == 0 {
        return (0, String::new());
    }

    (score.min(99), reasons.join(" + "))
}

fn match_text(candidate: &str, expected: &str, allow_partial: bool) -> bool {
    if allow_partial {
        candidate
            .to_ascii_lowercase()
            .contains(&expected.to_ascii_lowercase())
    } else {
        candidate.eq_ignore_ascii_case(expected)
    }
}

#[cfg(test)]
mod tests {
    use autoclick_domain::config::{FinderStrategies, TargetProfile};

    use super::score_window;
    use crate::window::{WindowInfo, WindowRect};

    fn sample_window() -> WindowInfo {
        WindowInfo {
            hwnd: 42,
            title: "Windsurf".to_string(),
            class_name: "Chrome_WidgetWin_1".to_string(),
            pid: 1,
            exe_path: Some("apps/Windsurf/Windsurf.exe".to_string()),
            is_minimized: false,
            is_visible: true,
            rect: WindowRect {
                left: 0,
                top: 0,
                right: 100,
                bottom: 100,
            },
        }
    }

    #[test]
    fn scores_exact_hwnd_highest() {
        let target = TargetProfile {
            hwnd: Some(42),
            process_name: None,
            process_path: None,
            title_contains: None,
            class_name: None,
            allow_partial_match: true,
            strategies: FinderStrategies::default(),
        };
        let (score, reason) = score_window(&sample_window(), &target);
        assert_eq!(score, 100);
        assert!(reason.contains("hwnd"));
    }

    #[test]
    fn skips_process_name_when_strategy_disabled() {
        let target = TargetProfile {
            hwnd: None,
            process_name: Some("Windsurf.exe".to_string()),
            process_path: None,
            title_contains: None,
            class_name: None,
            allow_partial_match: true,
            strategies: FinderStrategies {
                process_name: false,
                process_path: false,
                window_title: false,
                class_name: false,
                fuzzy_match: false,
            },
        };
        let (score, _) = score_window(&sample_window(), &target);
        assert_eq!(score, 0);
    }

    #[test]
    fn uses_enabled_title_strategy() {
        let target = TargetProfile {
            hwnd: None,
            process_name: None,
            process_path: None,
            title_contains: Some("surf".to_string()),
            class_name: None,
            allow_partial_match: true,
            strategies: FinderStrategies {
                process_name: false,
                process_path: false,
                window_title: true,
                class_name: false,
                fuzzy_match: false,
            },
        };
        let (score, reason) = score_window(&sample_window(), &target);
        assert_eq!(score, 20);
        assert!(reason.contains("标题"));
    }
}
