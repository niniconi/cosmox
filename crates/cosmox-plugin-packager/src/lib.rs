use anyhow::{Context, Result, anyhow};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
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

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
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

            log::debug!("copy file from {:?} to {:?}", src_file, dst_file);
            fs::copy(&src_file, &dst_file)?;
        } else if let Some(dst_parent) = dst.parent()
            && dst_parent.is_dir()
            && let Some(dst_dir) = src.file_name()
        {
            let dst_dir_path = dst_parent.join(dst_dir);
            let dst_file = dst_dir_path.join(file_path);
            fs::create_dir_all(dst_dir_path).context("")?;
            log::debug!("copy file from {:?} to {:?}", src_file, dst_file);

            fs::copy(&src_file, &dst_file)?;
        }
    }
    Ok(())
}

pub fn pack(
    src_path: impl AsRef<Path>,
    dst_path: impl AsRef<Path>,
    pack_profile: PackFromProfile,
) -> Result<()> {
    let repo_dir = PathBuf::from(src_path.as_ref());

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

    let target_plugin_dir = PathBuf::from(dst_path.as_ref())
        .join(profile)
        .join(repo_name);
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

    log::info!("--- Task Started: Building and Packaging WASM Plugin ---");

    // Create output directory
    fs::create_dir_all(&target_plugin_build_dir)?;

    // Task 1: Create Plugin Manifest File (yaml)
    log::info!("▶ Creating plugin manifest file...");

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

    log::info!("✅ Manifest file created: {}", about_file_name);

    // Task 2: Copy WASM
    log::info!("▶ Copy WASM file {:?}", repo_wasm_file_path);

    fs::create_dir_all(target_plugin_wasm_dir)?;
    fs::copy(repo_wasm_file_path, target_plugin_wasm_file_path)?;

    log::info!("✅ All WASM files copied.");

    // Task 3: Copy static files
    log::info!("▶ Copying static files...");
    copy_dir(&source_plugin_define_dir, &target_plugin_define_dir)?;
    copy_dir(&source_plugin_asset_dir, &target_plugin_asset_dir)?;
    log::info!("✅ All static files copied.");

    // Task 4: Package into a .tar.gz file
    log::info!("▶ Packaging files into a tar.gz archive...");
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
        .append_dir_all(repo_name, &target_plugin_build_dir)
        .with_context(|| {
            format!(
                "Failed to append directory {:?} to tar archive",
                target_plugin_build_dir
            )
        })?;
    tar_builder
        .finish()
        .with_context(|| "Failed to finalize tar.gz archive".to_string())?;
    log::info!(
        "✅ Successfully compressed {:?} to {:?}",
        target_plugin_build_dir,
        final_archive_file_path
    );

    Ok(())
}

/// Validates the archive structure and returns its single root directory path.
///
/// All entries within the archive must reside under this single root directory.
/// The root itself must be a directory, and no file or secondary directory is
/// permitted to exist at the root level.
///
/// # Errors
///
/// Returns an error if:
/// - The archive is empty.
/// - Any path traversal attempt (e.g., `..`) is detected.
/// - Multiple root directories or root-level files are found.
/// - The dynamically detected root is a file rather than a directory.
pub fn validate_archive_structure<P: AsRef<Path>>(archive_path: P) -> Result<PathBuf> {
    let tar_gz = File::open(&archive_path).with_context(|| {
        format!(
            "Failed to open archive: {:?}",
            archive_path.as_ref().display()
        )
    })?;

    let decoder = GzDecoder::new(tar_gz);
    let mut archive = tar::Archive::new(decoder);

    // Since Archive<T> requires exclusive access to iterate via entries(),
    // but we only have a shared reference `&Archive<T>`, we must use unchecked_entries().
    let entries = archive
        .entries()
        .context("Failed to read archive entries")?;

    let mut detected_root: Option<PathBuf> = None;

    for entry_result in entries {
        let entry = entry_result.context("Corrupted entry encountered in archive")?;
        let path = entry.path().context("Invalid path found in entry")?;

        if path.components().any(|c| c == Component::ParentDir) {
            return Err(anyhow!(
                "Security Warning: Path traversal detected in {:?}",
                path
            ));
        }

        let mut components = path.components();
        let first_component = match components.next() {
            Some(Component::Normal(p)) => Path::new(p),
            _ => return Err(anyhow!("Invalid path structure: {:?}", path)),
        };

        match &detected_root {
            None => {
                detected_root = Some(first_component.to_path_buf());

                if path == first_component && !entry.header().entry_type().is_dir() {
                    return Err(anyhow!(
                        "The root element '{:?}' must be a directory, but found a file.",
                        first_component
                    ));
                }
            }
            Some(root) => {
                if first_component != root {
                    return Err(anyhow!(
                        "Structure violation: Expected root '{:?}', but found '{:?}' in path {:?}",
                        root,
                        first_component,
                        path
                    ));
                }
            }
        }
    }

    detected_root.ok_or_else(|| anyhow!("Validation failed: The archive is empty."))
}

pub fn unpack(archive_path: impl AsRef<Path>, dst_path: impl AsRef<Path>) -> Result<PathBuf> {
    let archive_path = PathBuf::from(archive_path.as_ref());
    let dst = PathBuf::from(dst_path.as_ref());

    log::info!("--- Extracting plugin archive ---");
    log::info!("▶ Extracting {:?} to {:?}", archive_path, dst);

    let archive_root_directory = validate_archive_structure(&archive_path)?;
    let tar_gz = File::open(&archive_path)
        .with_context(|| format!("Failed to open archive: {:?}", archive_path))?;
    let decoder = GzDecoder::new(tar_gz);
    let mut archive_reader = tar::Archive::new(decoder);
    archive_reader
        .unpack(&dst)
        .map_err(|err| anyhow!("Failed to extract to {dst:?}: {err}"))?;

    log::info!("✅ Successfully extracted {:?} to {:?}", archive_path, dst);
    Ok(dst.join(archive_root_directory))
}
