//! Profile configuration data models and manager.
//! Manages loading, parsing, caching, and matching JSON profiles against target process executables.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Action definition associated with a slice or center hub.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionConfig {
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ring_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<ActionConfig>>,
}

/// Individual slice item representing an annular sector in the radial menu.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SliceItem {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionConfig>,
}

/// Submenu ring definition containing its own slice items.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SubmenuConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub slices: Vec<SliceItem>,
}

/// Full radial menu profile configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileConfig {
    pub profile_name: String,
    pub process_targets: Vec<String>,
    pub inner_radius: u32,
    pub outer_radius: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_action: Option<ActionConfig>,
    pub slices: Vec<SliceItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submenus: Option<HashMap<String, SubmenuConfig>>,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        ProfileConfig {
            profile_name: "Default Global".to_string(),
            process_targets: vec!["*".to_string()],
            inner_radius: 50,
            outer_radius: 140,
            center_action: None,
            slices: vec![
                SliceItem {
                    id: "slice_copy".to_string(),
                    label: "Copy".to_string(),
                    icon: Some("copy".to_string()),
                    action: Some(ActionConfig {
                        action_type: "Shortcut".to_string(),
                        keys: Some(vec!["Control".to_string(), "c".to_string()]),
                        url: None,
                        command: None,
                        text: None,
                        target_ring_id: None,
                        delay_ms: None,
                        actions: None,
                    }),
                },
                SliceItem {
                    id: "slice_paste".to_string(),
                    label: "Paste".to_string(),
                    icon: Some("clipboard".to_string()),
                    action: Some(ActionConfig {
                        action_type: "Shortcut".to_string(),
                        keys: Some(vec!["Control".to_string(), "v".to_string()]),
                        url: None,
                        command: None,
                        text: None,
                        target_ring_id: None,
                        delay_ms: None,
                        actions: None,
                    }),
                },
                SliceItem {
                    id: "slice_undo".to_string(),
                    label: "Undo".to_string(),
                    icon: Some("arrow-undo".to_string()),
                    action: Some(ActionConfig {
                        action_type: "Shortcut".to_string(),
                        keys: Some(vec!["Control".to_string(), "z".to_string()]),
                        url: None,
                        command: None,
                        text: None,
                        target_ring_id: None,
                        delay_ms: None,
                        actions: None,
                    }),
                },
                SliceItem {
                    id: "slice_redo".to_string(),
                    label: "Redo".to_string(),
                    icon: Some("arrow-redo".to_string()),
                    action: Some(ActionConfig {
                        action_type: "Shortcut".to_string(),
                        keys: Some(vec!["Control".to_string(), "y".to_string()]),
                        url: None,
                        command: None,
                        text: None,
                        target_ring_id: None,
                        delay_ms: None,
                        actions: None,
                    }),
                },
            ],
            submenus: None,
        }
    }
}

impl ProfileConfig {
    /// Returns the slice items for a specific ring ID ("root", empty, or a registered submenu).
    pub fn get_ring_slices(&self, ring_id: &str) -> Option<&[SliceItem]> {
        if ring_id.is_empty() || ring_id == "root" {
            Some(&self.slices)
        } else {
            self.submenus.as_ref()?.get(ring_id).map(|s| s.slices.as_slice())
        }
    }
    /// Determines whether this profile applies to the given process name.
    pub fn matches_process(&self, process_name: &str) -> bool {
        let clean_proc = normalize_target_name(process_name);
        if clean_proc.is_empty() {
            return false;
        }

        for target in &self.process_targets {
            let clean_target = normalize_target_name(target);
            if clean_target == "*" {
                return true;
            }
            if clean_target == clean_proc {
                return true;
            }
            // Match with or without .exe extension
            let proc_base = clean_proc.trim_end_matches(".exe");
            let target_base = clean_target.trim_end_matches(".exe");
            if !proc_base.is_empty() && proc_base == target_base {
                return true;
            }
        }
        false
    }
}

/// Normalizes a process or target string into a clean lowercase filename.
pub fn normalize_target_name(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed == "*" {
        return "*".to_string();
    }
    // Strip any preceding directory path components
    let path = Path::new(trimmed);
    let leaf = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(trimmed);
    leaf.to_lowercase()
}

/// Profile manager responsible for caching, directory scanning, and matching profiles.
#[derive(Debug, Clone)]
pub struct ProfileManager {
    profiles_dir: PathBuf,
    default_profile: ProfileConfig,
    /// Exact target process name (e.g. "code.exe") -> ProfileConfig
    process_map: HashMap<String, ProfileConfig>,
    /// Profile by name (e.g. "Visual Studio Code") -> ProfileConfig
    profiles_by_name: HashMap<String, ProfileConfig>,
    /// List of all loaded profiles
    all_profiles: Vec<ProfileConfig>,
}

impl ProfileManager {
    /// Creates a new ProfileManager initialized with the default profile.
    pub fn new(profiles_dir: impl Into<PathBuf>) -> Self {
        let dir = profiles_dir.into();
        Self {
            profiles_dir: dir,
            default_profile: ProfileConfig::default(),
            process_map: HashMap::new(),
            profiles_by_name: HashMap::new(),
            all_profiles: Vec::new(),
        }
    }

    /// Returns the directory path monitored for profiles.
    pub fn profiles_dir(&self) -> &Path {
        &self.profiles_dir
    }

    /// Sets a new profiles directory.
    pub fn set_profiles_dir(&mut self, dir: impl Into<PathBuf>) {
        self.profiles_dir = dir.into();
    }

    /// Returns a reference to the active default/fallback profile.
    pub fn default_profile(&self) -> &ProfileConfig {
        &self.default_profile
    }

    /// Sets the fallback default profile.
    pub fn set_default_profile(&mut self, profile: ProfileConfig) {
        self.default_profile = profile;
    }

    /// Returns a list of all loaded profiles.
    pub fn list_profiles(&self) -> &[ProfileConfig] {
        &self.all_profiles
    }

    /// Retrieves a profile by its name.
    pub fn get_profile_by_name(&self, name: &str) -> Option<&ProfileConfig> {
        let name_lower = name.trim().to_lowercase();
        self.profiles_by_name.get(&name_lower)
    }

    /// Scans the profiles directory and loads all JSON profile configurations.
    /// Returns the count of successfully loaded profiles.
    pub fn load_all(&mut self) -> Result<usize, String> {
        self.process_map.clear();
        self.profiles_by_name.clear();
        self.all_profiles.clear();

        if !self.profiles_dir.exists() {
            // Keep default fallback profile if directory doesn't exist
            self.all_profiles.push(self.default_profile.clone());
            self.profiles_by_name.insert(
                self.default_profile.profile_name.to_lowercase(),
                self.default_profile.clone(),
            );
            return Ok(1);
        }

        let entries = fs::read_dir(&self.profiles_dir)
            .map_err(|e| format!("Failed to read profiles directory {:?}: {}", self.profiles_dir, e))?;

        let mut loaded_count = 0;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<ProfileConfig>(&content) {
                        Ok(profile) => {
                            let is_default_file = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .map(|s| s.eq_ignore_ascii_case("default.json"))
                                .unwrap_or(false);

                            let has_wildcard = profile
                                .process_targets
                                .iter()
                                .any(|t| normalize_target_name(t) == "*");

                            if is_default_file || has_wildcard {
                                self.default_profile = profile.clone();
                            }

                            // Register process target mappings
                            for target in &profile.process_targets {
                                let norm = normalize_target_name(target);
                                if norm != "*" {
                                    self.process_map.insert(norm.clone(), profile.clone());
                                    let base = norm.trim_end_matches(".exe").to_string();
                                    if !base.is_empty() && base != norm {
                                        self.process_map.insert(base, profile.clone());
                                    }
                                }
                            }

                            self.profiles_by_name.insert(
                                profile.profile_name.trim().to_lowercase(),
                                profile.clone(),
                            );
                            self.all_profiles.push(profile);
                            loaded_count += 1;
                        }
                        Err(err) => {
                            eprintln!("Warning: could not parse profile at {:?}: {}", path, err);
                        }
                    },
                    Err(err) => {
                        eprintln!("Warning: could not read profile file {:?}: {}", path, err);
                    }
                }
            }
        }

        // Ensure default profile is represented in all_profiles if not present
        if !self.all_profiles.iter().any(|p| p.profile_name == self.default_profile.profile_name) {
            self.all_profiles.push(self.default_profile.clone());
        }

        Ok(loaded_count)
    }

    /// Finds the best matching profile for an application process name.
    /// Falls back to the default profile if no matching profile is configured.
    pub fn get_profile_for_process(&self, process_name: &str) -> ProfileConfig {
        let norm = normalize_target_name(process_name);
        if norm.is_empty() {
            return self.default_profile.clone();
        }

        // 1. Direct match in process map (exact filename or base)
        if let Some(profile) = self.process_map.get(&norm) {
            return profile.clone();
        }

        let base = norm.trim_end_matches(".exe");
        if let Some(profile) = self.process_map.get(base) {
            return profile.clone();
        }

        // 2. Iterate through loaded profiles and test matches_process
        for profile in &self.all_profiles {
            if profile.matches_process(&norm) {
                return profile.clone();
            }
        }

        // 3. Fallback to default profile
        self.default_profile.clone()
    }
}

/// Resolves the candidate directory for storing JSON profile configurations.
pub fn resolve_profiles_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("profiles"),
        PathBuf::from("../profiles"),
        PathBuf::from("../../profiles"),
    ];

    for c in &candidates {
        if c.exists() && c.is_dir() {
            if let Ok(canon) = c.canonicalize() {
                return canon;
            }
            return c.clone();
        }
    }

    PathBuf::from("profiles")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_target_name() {
        assert_eq!(normalize_target_name("*"), "*");
        assert_eq!(normalize_target_name("Code.exe"), "code.exe");
        assert_eq!(normalize_target_name("CODE.EXE"), "code.exe");
        assert_eq!(
            normalize_target_name(r"C:\Program Files\Microsoft VS Code\Code.exe"),
            "code.exe"
        );
        assert_eq!(normalize_target_name("notepad"), "notepad");
    }

    #[test]
    fn test_profile_matches_process() {
        let profile = ProfileConfig {
            profile_name: "VS Code".to_string(),
            process_targets: vec!["code.exe".to_string(), "code".to_string()],
            inner_radius: 50,
            outer_radius: 140,
            center_action: None,
            slices: vec![],
        };

        assert!(profile.matches_process("code.exe"));
        assert!(profile.matches_process("Code.exe"));
        assert!(profile.matches_process("CODE.EXE"));
        assert!(profile.matches_process("code"));
        assert!(profile.matches_process(r"C:\Program Files\VSCode\Code.exe"));
        assert!(!profile.matches_process("notepad.exe"));
        assert!(!profile.matches_process(""));
    }

    #[test]
    fn test_wildcard_matching() {
        let default_prof = ProfileConfig::default();
        assert!(default_prof.matches_process("any_app.exe"));
        assert!(default_prof.matches_process("blender.exe"));
        assert!(default_prof.matches_process("explorer.exe"));
    }

    #[test]
    fn test_profile_manager_matching_and_fallback() {
        let mut manager = ProfileManager::new(PathBuf::from("non_existent_dir_12345"));
        let _ = manager.load_all();

        // Should return default profile for unknown process
        let matched = manager.get_profile_for_process("unknown_app.exe");
        assert_eq!(matched.profile_name, "Default Global");

        // Manually inject a profile
        let code_profile = ProfileConfig {
            profile_name: "Visual Studio Code".to_string(),
            process_targets: vec!["code.exe".to_string()],
            inner_radius: 50,
            outer_radius: 150,
            center_action: None,
            slices: vec![],
            submenus: None,
        };
        manager.all_profiles.push(code_profile.clone());
        manager.process_map.insert("code.exe".to_string(), code_profile);

        let code_match = manager.get_profile_for_process(r"D:\apps\Code.exe");
        assert_eq!(code_match.profile_name, "Visual Studio Code");
        assert_eq!(code_match.outer_radius, 150);

        // Unknown process still falls back to default
        let fallback = manager.get_profile_for_process("chrome.exe");
        assert_eq!(fallback.profile_name, "Default Global");
    }

    #[test]
    fn test_profile_submenus_and_ring_slices() {
        let mut submenus = HashMap::new();
        submenus.insert(
            "git_tools".to_string(),
            SubmenuConfig {
                title: Some("Git Operations".to_string()),
                slices: vec![SliceItem {
                    id: "slice_git_status".to_string(),
                    label: "Git Status".to_string(),
                    icon: Some("terminal".to_string()),
                    action: Some(ActionConfig {
                        action_type: "Shortcut".to_string(),
                        keys: Some(vec!["Control".to_string(), "`".to_string()]),
                        url: None,
                        command: None,
                        text: None,
                        target_ring_id: None,
                        delay_ms: None,
                        actions: None,
                    }),
                }],
            },
        );

        let prof = ProfileConfig {
            profile_name: "Submenu Test".to_string(),
            process_targets: vec!["test.exe".to_string()],
            inner_radius: 50,
            outer_radius: 140,
            center_action: None,
            slices: vec![SliceItem {
                id: "slice_open_git".to_string(),
                label: "Git".to_string(),
                icon: Some("git".to_string()),
                action: Some(ActionConfig {
                    action_type: "Submenu".to_string(),
                    keys: None,
                    url: None,
                    command: None,
                    text: None,
                    target_ring_id: Some("git_tools".to_string()),
                    delay_ms: None,
                    actions: None,
                }),
            }],
            submenus: Some(submenus),
        };

        // Root slices
        let root = prof.get_ring_slices("root").unwrap();
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].id, "slice_open_git");

        // Submenu slices
        let git_ring = prof.get_ring_slices("git_tools").unwrap();
        assert_eq!(git_ring.len(), 1);
        assert_eq!(git_ring[0].id, "slice_git_status");

        // Non-existent submenu
        assert!(prof.get_ring_slices("invalid_ring").is_none());
    }
}
