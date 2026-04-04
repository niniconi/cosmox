use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::metadata::Metadata;

/// Represents the root structure of the entire Cargo.toml file
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")] // This is to enable mapping kebab-case fields in TOML to snake_case in Rust.
pub struct CargoToml {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub package: Option<Package>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub lib: Option<Lib>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bin: Option<Vec<Bin>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub example: Option<Vec<Example>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub test: Option<Vec<Test>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bench: Option<Vec<Bench>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub dependencies: Option<HashMap<String, Dependency>>,
  #[serde(rename = "dev-dependencies", skip_serializing_if = "Option::is_none")]
  pub dev_dependencies: Option<HashMap<String, Dependency>>,
  #[serde(rename = "build-dependencies", skip_serializing_if = "Option::is_none")]
  pub build_dependencies: Option<HashMap<String, Dependency>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub features: Option<HashMap<String, Vec<String>>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub target: Option<HashMap<String, Target>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub patch: Option<HashMap<String, HashMap<String, Dependency>>>, // Dependency overrides
  #[serde(skip_serializing_if = "Option::is_none")]
  pub replace: Option<HashMap<String, Dependency>>, // Dependency replacement (deprecated)
  #[serde(skip_serializing_if = "Option::is_none")]
  pub workspace: Option<Workspace>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub profile: Option<HashMap<String, Profile>>, // Build profile (dev, release, ..)
}

/// [package] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
  pub name: String,
  pub version: String,
  pub edition: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub license: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub license_file: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub readme: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub homepage: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub repository: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub documentation: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub keywords: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub categories: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub authors: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub maintainers: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exclude: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub include: Option<Vec<String>>,
  #[serde(default)] // publish Default is true
  #[serde(skip_serializing_if = "Option::is_none")]
  pub publish: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub build: Option<String>, // "build.rs" or a custom path
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rust_version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default_run: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub metadata: Option<Metadata>,
}

/// [lib] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Lib {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>, // Defaults to the package name
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>, // Default "src/lib.rs"
  #[serde(skip_serializing_if = "Option::is_none")]
  pub proc_macro: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub harness: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub required_features: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub doctest: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bench: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub test: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub doc: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub plugin: Option<bool>, // This is now deprecated.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub lto: Option<LtoConfig>, // Linking optimizations
  #[serde(skip_serializing_if = "Option::is_none")]
  pub panic: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rpath: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebugConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub strip: Option<StripConfig>,
}

/// [bin] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Bin {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>, // Default "src/main.rs" or "src/bin/<name>.rs"
  #[serde(skip_serializing_if = "Option::is_none")]
  pub required_features: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub harness: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub test: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bench: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub doc: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub lto: Option<LtoConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub panic: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rpath: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebugConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub strip: Option<StripConfig>,
}

/// [example] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Example {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>, // Default "examples/<name>.rs"
  #[serde(skip_serializing_if = "Option::is_none")]
  pub required_features: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub harness: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub test: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bench: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub doc: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub lto: Option<LtoConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub panic: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rpath: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebugConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub strip: Option<StripConfig>,
}

/// [test] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Test {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>, // Default "tests/<name>.rs"
  #[serde(skip_serializing_if = "Option::is_none")]
  pub required_features: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub harness: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub bench: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub doctest: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub lto: Option<LtoConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub panic: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rpath: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebugConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub strip: Option<StripConfig>,
}

/// [bench] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Bench {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>, // Default "benches/<name>.rs"
  #[serde(skip_serializing_if = "Option::is_none")]
  pub required_features: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub harness: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub test: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub lto: Option<LtoConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub panic: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rpath: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebugConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub strip: Option<StripConfig>,
}

/// Dependency definitions (can be a simple string or a table)
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)] // Attempt to match multiple structs in order.
pub enum Dependency {
  /// A simple version string, such as "0.8.5" or "=1.2.3".
  Version(String),
  /// Full table definition
  Table(DependencyTable),
}

/// Table definition for a dependency
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct DependencyTable {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub version: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub path: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub git: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub branch: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub tag: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rev: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub features: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default_features: Option<bool>, // Default is true
  #[serde(skip_serializing_if = "Option::is_none")]
  pub optional: Option<bool>, // Default is false
  #[serde(skip_serializing_if = "Option::is_none")]
  pub package: Option<String>, // If the crate name is different from the package name.
  #[serde(skip_serializing_if = "Option::is_none")]
  pub registry: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub registry_url: Option<String>,
}

/// [workspace] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Workspace {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub members: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub exclude: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default_members: Option<Vec<String>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub resolver: Option<String>, // "2" or "1"
}

/// [target.'cfg'] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Target {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub dependencies: Option<HashMap<String, Dependency>>,
  #[serde(rename = "dev-dependencies", skip_serializing_if = "Option::is_none")]
  pub dev_dependencies: Option<HashMap<String, Dependency>>,
  #[serde(rename = "build-dependencies", skip_serializing_if = "Option::is_none")]
  pub build_dependencies: Option<HashMap<String, Dependency>>,
}

/// [profile.*] part
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Profile {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub opt_level: Option<OptLevel>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug: Option<DebugConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub debug_assertions: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub overflow_checks: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub lto: Option<LtoConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub codegen_units: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub panic: Option<String>, // "abort" or "unwind"
  #[serde(skip_serializing_if = "Option::is_none")]
  pub rpath: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub incremental: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub strip: Option<StripConfig>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub split_debuginfo: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub compiler_builtins: Option<bool>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub build_override: Option<Box<Profile>>, // Allow overriding the build profile of a sub-dependency.
}

/// Optimize level (0, 1, 2, 3, "s", "z")
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum OptLevel {
  LevelNum(u8),
  LevelStr(String),
}

/// Debug info level (true, false, 0, 1, 2)
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum DebugConfig {
  Enabled(bool),
  Level(u8),
}

/// LTO Profile (true, false, "thin", "fat", "on", "off")
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum LtoConfig {
  Enabled(bool),
  Mode(String),
}

/// Strip Profile (true, false, "debuginfo", "symbols")
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(untagged)]
pub enum StripConfig {
  Enabled(bool),
  Mode(String),
}

#[cfg(test)]
mod tests {
  use crate::cargo::*;

  #[test]
  fn deserialize_full_cargo_toml() {
    let toml_str = r#"
            [package]
            name = "my-lib"
            version = "0.1.0"
            edition = "2021"
            description = "A test library"
            authors = ["Alice <alice@example.com>"]
            publish = true

            [lib]
            path = "src/mylib.rs"
            proc-macro = true

            [[bin]]
            name = "another-cli"
            path = "src/another.rs"
            required-features = ["full-features"]

            [dependencies]
            rand = "0.8.5"
            serde = { version = "1.0", features = ["derive"] }
            log = { version = "0.4", optional = true }

            [dev-dependencies]
            criterion = { version = "0.4", optional = true }

            [features]
            full-features = ["log"]
            serde-derive = ["serde/derive"]

            [target.'cfg(unix)']
            dependencies = { libc = "0.2" }

            [workspace]
            members = ["./crates/*"]
            exclude = ["./temp_crate"]
            resolver = "2"

            [profile.release]
            opt-level = 3
            debug = 0
            lto = "fat"
            codegen-units = 1
            strip = "debuginfo"

            [profile.dev.build-override]
            opt-level = "s"
        "#;

    let cargo_toml: CargoToml = toml::from_str(toml_str).expect("Failed to deserialize Cargo.toml");

    assert_eq!(cargo_toml.package.as_ref().unwrap().name, "my-lib");
    assert!(cargo_toml.lib.as_ref().unwrap().proc_macro.unwrap());
    assert_eq!(cargo_toml.bin.as_ref().unwrap().len(), 1);
    assert_eq!(
      cargo_toml
        .dependencies
        .as_ref()
        .unwrap()
        .get("rand")
        .unwrap(),
      &Dependency::Version("0.8.5".to_string())
    );
    assert!(matches!(
      cargo_toml
        .profile
        .as_ref()
        .unwrap()
        .get("release")
        .unwrap()
        .opt_level
        .as_ref()
        .unwrap(),
      OptLevel::LevelNum(3)
    ));
    assert!(
      matches!(cargo_toml.profile.as_ref().unwrap().get("release").unwrap().strip.as_ref().unwrap(), StripConfig::Mode(s) if s == "debuginfo")
    );
    assert_eq!(
      cargo_toml
        .workspace
        .as_ref()
        .unwrap()
        .resolver
        .as_ref()
        .unwrap(),
      "2"
    );

    // test deserialize
    let serialized_toml =
      toml::to_string_pretty(&cargo_toml).expect("Failed to serialize Cargo.toml");
    // You can't directly compare strings here because the order of TOML entries might differ. Instead, you can compare the structs after deserialization.
    let deserialized_again: CargoToml =
      toml::from_str(&serialized_toml).expect("Failed to deserialize serialized TOML");
    assert_eq!(cargo_toml, deserialized_again);
  }
}
