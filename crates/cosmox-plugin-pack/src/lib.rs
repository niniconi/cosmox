use anyhow::{Context, Result};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::Builder;

use crate::cargo::CargoToml;

mod cargo;
mod metadata;

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

pub enum PackFromProfile {
    Debug,
    Release,
}

macro_rules! info{
    ($fmt:literal $(, $args:expr)*) => {
        println!(concat!("\n▶ ", $fmt), $($args)*);
    };
}

macro_rules! complete {
    ($fmt:literal $(, $args:expr)*) => {
        println!(concat!("\n✅ ", $fmt), $($args)*);
    };
}

fn copy_dir(src: &PathBuf, dst: &Path) -> Result<()> {
    let mut src_files: Vec<PathBuf> = vec![];

    // find all src file
    if src.is_dir()
        && let Some(path) = src.to_str()
    {
        let mut dirs = vec![String::from(path)];
        while let Some(dir) = dirs.pop() {
            let entrys = fs::read_dir(dir).unwrap();

            for entry in entrys {
                if let Ok(entry) = entry
                    && let Ok(metadata) = entry.metadata()
                {
                    if metadata.is_dir() {
                        if let Some(path) = entry.path().to_str() {
                            dirs.push(String::from(path));
                        }
                    } else if metadata.is_file()
                        && let Some(path) = entry.path().to_str()
                    {
                        src_files.push(PathBuf::from(path));
                    }
                }
            }
        }
    }

    for src_file in src_files {
        let file_path = &src_file.strip_prefix(src)?;

        if dst.exists() && dst.is_dir() {
            let dst_file = dst.join(file_path);

            println!("copy file from {:?} to {:?}", src_file, dst_file);
            fs::copy(&src_file, &dst_file)?;
        } else if let Some(dst_parent) = dst.parent()
            && dst_parent.is_dir()
            && let Some(dst_dir) = src.file_name()
        {
            let dst_dir_path = dst_parent.join(dst_dir);
            let dst_file = dst_dir_path.join(file_path);
            fs::create_dir_all(dst_dir_path).context("")?;
            println!("copy file from {:?} to {:?}", src_file, dst_file);

            fs::copy(&src_file, &dst_file)?;
        }
    }
    Ok(())
}

pub fn pack(src_path: &str, dst_path: &str, pack_profile: PackFromProfile) -> Result<()> {
    let repo_dir = PathBuf::from(src_path);

    let raw_cargo_toml = fs::read_to_string(repo_dir.join("Cargo.toml")).unwrap();
    let cargo_toml: CargoToml = toml::from_str(raw_cargo_toml.as_str())?;
    let cargo_package = cargo_toml.package.unwrap();

    let repo_name = &cargo_package.name;
    let repo_description = &cargo_package.description;
    let repo_author = &cargo_package.authors.unwrap();
    let repo_license = &cargo_package.license.unwrap_or("custom".to_string());
    const WASM_TARGET_TRIPLE: &str = "wasm32-wasip2";
    let plugin_version = &cargo_package.version;
    let (dependencies, conflicts) = if let Some(metadata) = &cargo_package.metadata {
        (metadata.dependencies.clone(), metadata.conflicts.clone())
    } else {
        (None, None)
    };

    let profile = match pack_profile {
        PackFromProfile::Debug => "debug",
        PackFromProfile::Release => "release",
    };

    let source_plugin_asset_dir = repo_dir.join("assets");
    let source_plugin_define_dir = repo_dir.join("defines");

    let target_plugin_dir = PathBuf::from(dst_path).join(profile).join(repo_name);
    let target_plugin_build_dir = target_plugin_dir.join("build");
    let target_plugin_wasm_dir = target_plugin_build_dir.join("wasm");
    let target_plugin_define_dir = target_plugin_build_dir.join("defines");
    let target_plugin_asset_dir = target_plugin_build_dir.join("assets");

    // Generated plugin package file name
    let archive_filename = format!("{}-{}.tar.gz", repo_name, plugin_version);
    let final_archive_file_path = target_plugin_dir.join(&archive_filename);

    // Get the path to the compiled WASM file
    let wasm_file_name = format!("{}.wasm", repo_name.replace("-", "_"));
    let repo_wasm_file_path = repo_dir
        .join("target")
        .join(WASM_TARGET_TRIPLE)
        .join(profile)
        .join(&wasm_file_name);
    let target_plugin_wasm_file_path = target_plugin_wasm_dir.join(&wasm_file_name);

    println!("\n--- Task Started: Building and Packaging WASM Plugin ---");

    // Create output directory
    fs::create_dir_all(&target_plugin_build_dir)?;

    // ------------------------------------------------------------
    // Task 1: Create Plugin Manifest File (yaml)
    // ------------------------------------------------------------
    info!("Creating plugin manifest file...");

    let manifest = About {
        name: repo_name.clone(),
        version: plugin_version.clone(),
        description: repo_description.clone(),
        license: repo_license.clone(),
        authors: repo_author.clone(),
        dependencies,
        conflicts,
        ..Default::default()
    };

    let about_yaml = serde_yaml::to_string(&manifest)?;
    let about_file_name = "about.yaml";
    let about_file_path = target_plugin_build_dir.join(about_file_name);

    let mut about_file = File::create(about_file_path).context(format!(
        "Failed to write manifest file to {:?}",
        about_file_name
    ))?;
    write!(about_file, "{about_yaml}")?;

    complete!("Manifest file created: {}", about_file_name);

    // ------------------------------------------------------------
    // Task 2: Copy WASM
    // ------------------------------------------------------------
    info!("Copy WASM file {:?}", repo_wasm_file_path);

    fs::create_dir_all(target_plugin_wasm_dir)?;
    fs::copy(repo_wasm_file_path, target_plugin_wasm_file_path)?;

    complete!("All WASM files copied.");

    // ------------------------------------------------------------
    // Task 3: Copy static files
    // ------------------------------------------------------------
    info!("Copying static files...");
    copy_dir(&source_plugin_define_dir, &target_plugin_define_dir)?;
    copy_dir(&source_plugin_asset_dir, &target_plugin_asset_dir)?;
    complete!("All static files copied.");

    // ------------------------------------------------------------
    // Task 4: Package into a .tar.gz file
    // ------------------------------------------------------------
    info!("Packaging files into a tar.gz archive...");
    if !target_plugin_build_dir.is_dir() {
        return Err(anyhow::anyhow!(
            "Source path must be a directory: {:?}",
            target_plugin_build_dir
        ));
    }

    let tar_gz = File::create(&final_archive_file_path).with_context(|| {
        format!(
            "Failed to create output .tar.gz file: {:?}",
            final_archive_file_path
        )
    })?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar_builder = Builder::new(enc);
    tar_builder
        .append_dir_all(".", &target_plugin_build_dir)
        .with_context(|| {
            format!(
                "Failed to append directory {:?} to tar archive",
                target_plugin_build_dir
            )
        })?;
    tar_builder
        .finish()
        .with_context(|| "Failed to finalize tar.gz archive".to_string())?;
    println!(
        "Successfully compressed {:?} to {:?}",
        target_plugin_build_dir, final_archive_file_path
    );

    Ok(())
}
