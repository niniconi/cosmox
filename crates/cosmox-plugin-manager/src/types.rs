use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::{Debug, Display},
    ops::{AddAssign, Deref},
    path::PathBuf,
    sync::Arc,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use wasmtime::component::{Component, Linker};

use crate::plugin_loader::ComponentRunStates;

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub struct PluginId(u64);

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct PluginName(String);

#[derive(Debug, Default, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PluginWasmId(u64);

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct PluginWasmName(String);

pub type Plugin = Arc<PluginRaw>;

pub struct WasmComponent {
    pub id: PluginWasmId,
    pub name: PluginWasmName,
    pub plugin_id: PluginId,
    pub path: PathBuf,
    pub component: Component,
    pub linker: Arc<Linker<ComponentRunStates>>,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum PluginRaw {
    ExternalPlugin {
        id: PluginId,
        version: Version,
        name: PluginName,
        description: String,
        author: String,
        email: String,
        permission: Vec<String>,
        wasm_extensions: Option<HashMap<PluginWasmId, Arc<WasmComponent>>>,
        wasm_ui_extensions: Option<HashMap<PluginWasmId, Arc<WasmUiExtension>>>,
        dependencies: Option<Vec<Dependency>>,
        conflicts: Option<Vec<Dependency>>,
    },

    BuiltinPlugin {
        id: PluginId,
        version: Version,
        name: PluginName,
        description: String,
    },
}

#[derive(Debug)]
pub struct WasmUiExtension {
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct About {
    pub name: String,
    pub version: String,
    pub license: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub permission: Option<Vec<String>>,
    pub url: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub conflicts: Option<Vec<String>>,
}

#[derive(Debug, Clone, Eq)]
pub struct Version(Vec<String>);

/// Represents a semantic versioning requirement used to filter or match versions.
#[derive(Debug, Clone, PartialEq)]
pub enum VersionRequirement {
    /// Matches only the exact version specified.
    /// Syntax: `1.2.3` (No operator)
    Exact(Version),

    /// Matches any version greater than or equal to the specified version.
    /// Syntax: `>=1.2.3`
    GreaterEqual(Version),

    /// Matches any version less than or equal to the specified version.
    /// Syntax: `<=1.2.3`
    LessEqual(Version),

    /// Allows semver-compatible updates (The `^` operator).
    ///
    /// Rule: Allows changes that do not modify the left-most non-zero digit.
    /// - `^1.2.3`  =>  `[1.2.3, 2.0.0)`
    /// - `^0.2.3`  =>  `[0.2.3, 0.3.0)`
    /// - `^0.0.3`  =>  `[0.0.3, 0.0.4)`
    Caret(Version),

    /// Allows limited patch-level updates (The `~` operator).
    ///
    /// Rule: Allows patch increments if minor is specified; allows minor increments otherwise.
    /// - `~1.2.3`  =>  `[1.2.3, 1.3.0)`
    /// - `~1.2`    =>  `[1.2.0, 1.3.0)`
    /// - `~1`      =>  `[1.0.0, 2.0.0)`
    Tilde(Version),

    /// Matches any version (Wildcard).
    /// Syntax: `*`
    Any,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dependency {
    /// Plugin identifier (ID).
    ///
    /// This value might not be initialized at the time the `Dependency` is created.
    pub id: Option<PluginId>,

    pub name: PluginName,
    pub requirement: VersionRequirement,

    raw_dependency: String,
}

impl PluginRaw {
    pub fn id(&self) -> PluginId {
        match self {
            Self::BuiltinPlugin { id, .. } => *id,
            Self::ExternalPlugin { id, .. } => *id,
        }
    }

    pub fn name(&self) -> PluginName {
        match self {
            Self::BuiltinPlugin { name, .. } => name.clone(),
            Self::ExternalPlugin { name, .. } => name.clone(),
        }
    }
    pub fn version(&self) -> &Version {
        match self {
            Self::BuiltinPlugin { version, .. } => version,
            Self::ExternalPlugin { version, .. } => version,
        }
    }
}

impl Version {
    pub fn new(version: Vec<String>) -> Self {
        Self(version)
    }

    pub fn into_inner(&self) -> &[String] {
        self.0.as_ref()
    }

    /// Check if the version satisfies the ^ (Caret) requirement.
    pub fn matches_caret(&self, required: &Version) -> bool {
        let req_parts = &required.0;
        let self_parts = &self.0;

        if self_parts.len() < req_parts.len() {
            return false;
        }

        let first_non_zero = req_parts.iter().position(|s| s != "0").unwrap_or(0);

        // Lock up to the first non-zero item: the preceding parts must match exactly.
        for i in 0..=first_non_zero {
            if i < req_parts.len() && i < self_parts.len() && self_parts[i] != req_parts[i] {
                return false;
            }
        }

        // For items after the first non-zero, the version must be >= required.
        for i in (first_non_zero + 1)..req_parts.len() {
            let req_val = req_parts[i].parse::<u64>().unwrap_or(0);
            let self_val = self_parts[i].parse::<u64>().unwrap_or(0);

            if self_val > req_val {
                return true;
            } else if self_val < req_val {
                return false;
            }
        }

        true
    }

    /// Check if the version satisfies the ~ (Tilde) requirement.
    pub fn matches_tilde(&self, required: &Version) -> bool {
        let req_parts = &required.0;
        let self_parts = &self.0;

        if self_parts.len() < req_parts.len() {
            return false;
        }

        // Lock all items except the very last one.
        let last_idx = req_parts.len() - 1;
        for i in 0..last_idx {
            if self_parts[i] != req_parts[i] {
                return false;
            }
        }

        // The last item must be greater than or equal to the requirement.
        let req_val = req_parts[last_idx].parse::<u64>().unwrap_or(0);
        let self_val = self_parts[last_idx].parse::<u64>().unwrap_or(0);

        self_val >= req_val
    }
}

impl PluginId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl PluginWasmId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl PluginName {
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self(name.into())
    }
}

impl PluginWasmName {
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self(name.into())
    }
}

impl Dependency {
    pub fn parse<T: Into<String>>(dependency: T) -> Result<Dependency> {
        let dependency = dependency.into();
        let parts: Vec<&str> = dependency.split('@').collect();
        if parts.len() == 1 || (parts.len() == 2 && matches!(parts[1].trim(), "*" | "any")) {
            return Ok(Dependency {
                id: None,
                name: PluginName::new(parts[0]),
                requirement: VersionRequirement::Any,
                raw_dependency: dependency,
            });
        } else if parts.len() != 2 {
            anyhow::bail!("Invalid dependency format {dependency}. Use name@1.0.0");
        }

        let name = PluginName::new(parts[0].trim().to_string());
        let version_raw = parts[1].trim();
        if version_raw.is_empty() {
            anyhow::bail!("Invalid dependency format {dependency}. Use name@1.0.0");
        }

        let (operator_fn, v_num_str): (fn(Version) -> VersionRequirement, &str) =
            if let Some(version) = version_raw.strip_prefix(">=") {
                (VersionRequirement::GreaterEqual, version)
            } else if let Some(version) = version_raw.strip_prefix("<=") {
                (VersionRequirement::LessEqual, version)
            } else if let Some(version) = version_raw.strip_prefix('^') {
                (VersionRequirement::Caret, version)
            } else if let Some(version) = version_raw.strip_prefix('~') {
                (VersionRequirement::Tilde, version)
            } else {
                (VersionRequirement::Exact, version_raw)
            };

        let version = Version::from(v_num_str);

        Ok(Dependency {
            id: None,
            name,
            requirement: operator_fn(version),
            raw_dependency: dependency,
        })
    }
}

impl Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PluginId({})", self.0)
    }
}

impl Display for PluginWasmId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PluginWasmId({})", self.0)
    }
}

impl Display for PluginName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for PluginWasmName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Display for Dependency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_dependency)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, part) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{}", part)?;
        }
        Ok(())
    }
}

impl Debug for WasmComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "WasmComponent {{ id: {}, plugin_id: {} }}",
            self.id, self.plugin_id
        )
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0 == other.0 {
            Ordering::Equal
        } else {
            let mut v1 = self.0.iter();
            let mut v2 = other.0.iter();
            loop {
                match (v1.next(), v2.next()) {
                    (None, None) => return Ordering::Equal,
                    (Some(v1), Some(v2)) => {
                        let v1 = v1.parse::<u64>().unwrap();
                        let v2 = v2.parse::<u64>().unwrap();
                        if v1 > v2 {
                            return Ordering::Greater;
                        } else if v1 < v2 {
                            return Ordering::Less;
                        }
                    }
                    (None, Some(v)) => {
                        if v.parse::<u64>().unwrap() != 0 {
                            return Ordering::Less;
                        }
                        for v in v2 {
                            let v = v.parse::<u64>().unwrap();
                            if v != 0 {
                                return Ordering::Less;
                            }
                        }
                        return Ordering::Equal;
                    }
                    (Some(v), None) => {
                        if v.parse::<u64>().unwrap() != 0 {
                            return Ordering::Greater;
                        }
                        for v in v1 {
                            let v = v.parse::<u64>().unwrap();
                            if v != 0 {
                                return Ordering::Greater;
                            }
                        }
                        return Ordering::Equal;
                    }
                }
            }
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl AddAssign for PluginId {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl AddAssign for PluginWasmId {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl From<PluginId> for usize {
    fn from(value: PluginId) -> Self {
        value.0 as usize
    }
}

impl From<&PluginId> for usize {
    fn from(value: &PluginId) -> Self {
        usize::from(*value)
    }
}

impl From<PluginWasmId> for usize {
    fn from(value: PluginWasmId) -> Self {
        value.0 as usize
    }
}

impl From<&PluginWasmId> for usize {
    fn from(value: &PluginWasmId) -> Self {
        usize::from(*value)
    }
}

impl<T> From<T> for Version
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        let version = value.into();
        Version::new(
            version
                .trim()
                .split('.')
                .map(|x| x.trim().to_string())
                .collect(),
        )
    }
}

impl Deref for PluginId {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for PluginName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl Deref for PluginWasmId {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for PluginWasmName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing_from_str_and_string() {
        let v_str = Version::from("1.2.3");
        let v_string = Version::from("1.2.3".to_string());
        assert_eq!(v_str, v_string);
    }

    #[test]
    fn test_version_numeric_vs_lexicographical_ordering() {
        let v_small = Version::from("1.2.9");
        let v_big = Version::from("1.10.2");

        assert!(v_big > v_small);
        assert!(v_small < v_big);
    }

    #[test]
    fn test_version_different_lengths_comparison() {
        let v_short = Version::from("1.2");
        let v_long = Version::from("1.2.0");
        let v_longer = Version::from("1.2.0.1");

        assert!(v_long >= v_short);
        assert!(v_longer > v_long);
        assert!(v_longer > v_short);
    }

    #[test]
    fn test_version_leading_zeros_and_identity() {
        let v1 = Version::from("1.02.3");
        let v2 = Version::from("1.2.3");
        let v3 = Version::from("01.2.3");

        assert_eq!(v1, v2);
        assert_eq!(v2, v3);
    }

    #[test]
    fn test_version_extreme_values() {
        let v_max = Version::from("999999999999.99999999.999999");
        let v_min = Version::from("0.0.0");

        assert!(v_max > v_min);
    }

    #[test]
    fn test_version_trailing_zeros_equality() {
        let v_short = Version::from("1.2");
        let v_zeros = Version::from("1.2.0.0");
        let v_longer_zeros = Version::from("1.2.0.0.0");

        assert_eq!(v_short, v_zeros);
        assert_eq!(v_zeros, v_longer_zeros);
        assert!(v_short >= v_zeros);
        assert!(v_zeros <= v_short);
    }

    #[test]
    fn test_version_complex_trailing_zeros_comparison() {
        let v_base = Version::from("1.2.0");
        let v_more = Version::from("1.2.0.1");
        let v_less = Version::from("1.2.0.0.9");

        assert!(v_more > v_base);
        assert!(v_less > v_base);
        assert!(v_more > v_less);
    }

    #[test]
    fn test_version_zeros_strings() {
        let v_zero = Version::from("0");
        let v_triple_zero = Version::from("0.0.0");

        assert_eq!(v_zero, v_triple_zero);
    }

    #[test]
    fn test_version_caret_matching() {
        // ^1.2.3
        let req = Version::from("1.2.3");
        assert!(Version::from("1.2.3").matches_caret(&req));
        assert!(Version::from("1.2.4").matches_caret(&req));
        assert!(Version::from("1.3.0").matches_caret(&req));
        assert!(!Version::from("2.0.0").matches_caret(&req));
        assert!(!Version::from("1.2").matches_caret(&req));

        // ^0.2.3
        let zero_req = Version::from("0.2.3");
        assert!(Version::from("0.2.3").matches_caret(&zero_req));
        assert!(Version::from("0.2.9").matches_caret(&zero_req));
        assert!(!Version::from("0.3.0").matches_caret(&zero_req));

        // ^0.0.3
        let tiny_req = Version::from("0.0.3");
        assert!(Version::from("0.0.3").matches_caret(&tiny_req));
        assert!(!Version::from("0.0.4").matches_caret(&tiny_req));
    }

    #[test]
    fn test_version_tilde_matching() {
        // ~1.2.3
        let req = Version::from("1.2.3");
        assert!(Version::from("1.2.3").matches_tilde(&req));
        assert!(Version::from("1.2.9").matches_tilde(&req));
        assert!(!Version::from("1.3.0").matches_tilde(&req));
        assert!(!Version::from("1.1.9").matches_tilde(&req));

        // ~1.2
        let short_req = Version::from("1.2");
        assert!(Version::from("1.2").matches_tilde(&short_req));
        assert!(Version::from("1.5").matches_tilde(&short_req));
        assert!(!Version::from("2.0").matches_tilde(&short_req));
    }

    #[test]
    fn test_version_long_matching() {
        let deep_req = Version::from("1.2.3.4");
        assert!(Version::from("1.2.3.10").matches_tilde(&deep_req));
        assert!(Version::from("1.2.10.4").matches_caret(&deep_req));
    }
}
