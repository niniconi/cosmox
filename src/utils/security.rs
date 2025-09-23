use std::{
  borrow::Cow,
  collections::HashMap,
  ffi::OsString,
  path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result};

/// check if paths overlap
pub fn check_if_paths_overlap(base_dir: PathBuf, paths: Vec<&str>) -> bool {
  let mut cnts: HashMap<Cow<OsString>, u32> = HashMap::new();

  for path in paths {
    let mut path = PathBuf::from(path);

    if path.is_absolute() {
      if path.starts_with(&base_dir)
        && let Ok(relative_path) = path.strip_prefix(&base_dir)
      {
        path = PathBuf::from(relative_path);
      } else {
        break;
      }
    }

    let mut components = path.components();
    let component_option = components.next();

    if let Some(component) = component_option {
      let os_string = component.as_os_str().to_os_string();
      let key = Cow::Owned(os_string);
      if let Some(cnt) = cnts.get(&key) {
        cnts.insert(key, cnt + 1);
      } else {
        cnts.insert(key, 1);
      }
    }
  }

  let count = cnts.iter().filter(|(_, cnt)| **cnt == 1).count();

  count == 0
}

/// check if path safe
///
/// Checks if the user-provided filename is safe and does not contain path traversal characters or separators.
/// This is the first line of defense.
/// Checks if the final path, created by combining the given `user_path_component` (e.g., a filename or a single directory name)
/// and `base_dir`, is safely located under `base_dir`.
/// It accounts for the possibility that the file may not exist and provides more robust protection against path traversal.
///
/// # Arguments
/// - `base_dir`: The root directory where the application is allowed to operate. It is recommended to pass a `&Path` reference.
/// - `user_path_component`: The user-provided part of the path, such as a filename or a single subdirectory name.
///   This string should have already been validated by `is_safe_filename_component`.
///
/// # Returns
/// - `Ok(PathBuf)` If the path is safe and valid, returns the sanitized absolute path.
/// - `Err(anyhow Error)` If the path is unsafe or an error occurred during processing.
pub fn check_if_path_safe<P: AsRef<Path>>(
  base_dir: P,
  user_path_component: &str,
) -> Result<PathBuf> {
  let base_dir = base_dir.as_ref();
  if user_path_component.contains('/')
    || user_path_component.contains('\\')
    || user_path_component.contains("..")
    || user_path_component.contains('\0')
  // && filename.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
  {
    return Err(anyhow::anyhow!(
      "Invalid characters or sequence in user path component: '{}'",
      user_path_component
    ));
  }
  // 0. Validate if the user-provided path component is safe.
  // if !is_safe_filename_component(user_path_component) {
  //     return Err(anyhow::anyhow!("Invalid characters or sequence in user path component: '{}'", user_path_component));
  // }

  // 1. Combine the base directory and the user-provided path component.
  let tentative_path = base_dir.join(user_path_component);

  // 2. Normalize the base directory to an absolute path.
  // This will return an error if `base_dir` does not exist or is inaccessible.
  let abs_base_dir = base_dir.canonicalize().context(format!(
    "Failed to canonicalize base directory: '{}'. Does it exist and have permissions?",
    base_dir.display()
  ))?;

  // 3. Clean the combined path, removing '.' and '..'
  // path-clean does not resolve symbolic links or check for absolute path bypasses,
  // but it handles cases like "dir/../file.txt".
  // let cleaned_path = tentative_path.clean();
  let cleaned_path = clean_path_native(&tentative_path);

  // 4. Convert to the final absolute path.
  // Note: `canonicalize()` will still fail if the file or directory pointed to by `cleaned_path` does not exist.
  // Here, we assume that even if the file doesn't exist, its parent directory chain must exist for `canonicalize` to partially succeed.
  // A safer approach is that if `canonicalize` fails, you must accept that it has failed, or manually build and validate.
  let final_absolute_path = cleaned_path.canonicalize().context(format!(
    "Failed to canonicalize combined path '{}'. Does it exist or is it invalid?",
    cleaned_path.display()
  ))?;

  // 5. Check if the final absolute path has the canonicalized base directory as a prefix.
  // This is a critical step to prevent path injection.
  if !final_absolute_path.starts_with(&abs_base_dir) {
    return Err(anyhow::anyhow!(
      "Path traversal detected! Resulting path '{}' is not within base directory '{}'.",
      final_absolute_path.display(),
      abs_base_dir.display()
    ));
  }

  Ok(final_absolute_path)
}

/// Mimics the functionality of `path-clean`'s `PathClean::clean()`,
/// removing '.' and '..' components from the path without performing file system checks.
///
/// # Arguments
/// - `path`: The path to be cleaned.
///
/// # Returns
/// - The cleaned `PathBuf`.
fn clean_path_native<P: AsRef<Path>>(path: P) -> PathBuf {
  let path = path.as_ref();
  let mut cleaned_components = Vec::new();

  for component in path.components() {
    match component {
      Component::CurDir => {
        // If it's a root directory, or the previous one was a root directory, keep a '.' to prevent an empty path
        if cleaned_components.is_empty() && path.is_relative() {
          cleaned_components.push(Component::CurDir);
        }
      }
      // Handle the parent directory '..'
      Component::ParentDir => {
        // If there is a non-root component before, pop it
        // Otherwise, keep the '..' because we cannot go beyond the file system root
        if let Some(last_component) = cleaned_components.last() {
          if *last_component != Component::ParentDir {
            cleaned_components.pop();
          } else {
            // If the previous one was '..', this '..' should also be kept
            cleaned_components.push(Component::ParentDir);
          }
        } else {
          // If the list is empty and the path is relative, '..' must be kept
          // For example: "../a" should be cleaned to "../a"
          cleaned_components.push(Component::ParentDir);
        }
      }
      // Keep normal components (Normal, Prefix, RootDir)
      _ => {
        cleaned_components.push(component);
      }
    }
  }

  // Rebuild `PathBuf` based on the cleaned components
  let mut cleaned_path_buf = PathBuf::new();
  for component in cleaned_components {
    cleaned_path_buf.push(component);
  }

  cleaned_path_buf
}

#[cfg(test)]
mod test {
  use std::path::PathBuf;

  use crate::utils::security::{check_if_path_safe, check_if_paths_overlap};

  #[test]
  fn check_if_path_safe1() {
    let base_dir = &PathBuf::from("/home/administrator");
    let result = check_if_path_safe(base_dir, "Desktop");
    assert_eq!(result.is_ok(), true)
  }
  #[test]
  fn check_if_path_safe2() {
    let base_dir = &PathBuf::from("/home/administrator");
    let result = check_if_path_safe(base_dir, "..");
    assert_eq!(result.is_err(), true)
  }
  #[test]
  fn check_if_path_safe3() {
    let base_dir = &PathBuf::from("/home/administrator");
    let result = check_if_path_safe(base_dir, "../../");
    assert_eq!(result.is_err(), true)
  }

  #[test]
  fn check_if_path_safe4() {
    let base_dir = &PathBuf::from("/home/administrator");
    let result = check_if_path_safe(base_dir, "a/b/c/d/e/f");
    assert_eq!(result.is_ok(), false)
  }

  #[test]
  fn check_if_paths_overlap1() {
    let base_dir = PathBuf::from("/var/cache");
    let paths = vec![
      "abs/b/c/d",
      "a/b/",
      "public/vo",
      "object/a/b/c",
      "/var/cache/fun",
      "/var/cache/jojo",
      "/var/tmp/jojo",
    ];

    assert_eq!(check_if_paths_overlap(base_dir, paths), false)
  }
}
