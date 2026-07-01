use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};

/// Check that a path is safe within the base directory without requiring the target to exist.
///
/// `base` must exist on the filesystem (it is canonicalised to resolve symlinks).
/// Walks `input`'s path components relative to `base`, rejecting `..` that would
/// escape the base directory and absolute components (`RootDir`, `Prefix`) outright.
/// No filesystem access is performed on the resulting path, so the target file
/// does **not** need to exist.
///
/// # Errors
///
/// Returns an error if `base` is inaccessible or the resulting path escapes
/// the base directory.
pub fn check_new_path_safe<P: AsRef<Path>>(base: P, input: &str) -> Result<PathBuf> {
    let abs_base = base.as_ref().canonicalize().context(format!(
        "Failed to canonicalize base directory: '{}'. Does it exist and have permissions?",
        base.as_ref().display()
    ))?;

    let mut result = abs_base.clone();
    for component in Path::new(input).components() {
        match component {
            // Absolute path in input — would replace entire base via Path::join
            Component::RootDir | Component::Prefix(..) => {
                return Err(anyhow::anyhow!(
                    "Path traversal detected! Absolute path component in input '{}' escapes base directory '{}'.",
                    input,
                    abs_base.display()
                ));
            }
            Component::CurDir => {
                // '.' — no-op
            }
            Component::ParentDir => {
                // '..' — go up one level; if we leave base, reject
                result.pop();
                if !result.starts_with(&abs_base) {
                    return Err(anyhow::anyhow!(
                        "Path traversal detected! Input '{}' escapes base directory '{}'.",
                        input,
                        abs_base.display()
                    ));
                }
            }
            Component::Normal(_) => {
                result.push(component);
            }
        }
    }

    // Final containment safety check
    if !result.starts_with(&abs_base) {
        return Err(anyhow::anyhow!(
            "Path traversal detected! Input '{}' escapes base directory '{}'.",
            input,
            abs_base.display()
        ));
    }

    Ok(result)
}

/// Verify that an existing file at the given path is safe within the base directory.
///
/// Both `base` and the resolved target path must exist on the filesystem.
/// Uses `canonicalize` on both to resolve symlinks — this inherently resolves
/// all `.` and `..` components, so no separate path cleaning is needed.
///
/// # Errors
///
/// Returns an error if `base` is inaccessible, the target file does not exist,
/// or the resolved path escapes the base directory.
pub fn check_existing_path_safe<P: AsRef<Path>>(base: P, input: &str) -> Result<PathBuf> {
    let abs_base = base.as_ref().canonicalize().context(format!(
        "Failed to canonicalize base directory: '{}'. Does it exist and have permissions?",
        base.as_ref().display()
    ))?;

    let tentative = abs_base.join(input);

    // canonicalize resolves all . and .. components natively
    let resolved = tentative.canonicalize().with_context(|| {
        format!(
            "Target path '{}' does not exist or is inaccessible",
            tentative.display()
        )
    })?;

    if !resolved.starts_with(&abs_base) {
        return Err(anyhow::anyhow!(
            "Path traversal detected! Resolved path '{}' is not within base directory '{}'.",
            resolved.display(),
            abs_base.display()
        ));
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn create_test_dir(name: &str) -> PathBuf {
        let root = PathBuf::from("/tmp/security_test").join(name);
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn remove_test_dir(name: &str) {
        let root = PathBuf::from("/tmp/security_test").join(name);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn new_path_accepts_simple_filename() {
        let base = create_test_dir("new_valid");
        let result = check_new_path_safe(&base, "new_file.txt");
        assert!(result.is_ok());
        remove_test_dir("new_valid");
    }

    #[test]
    fn new_path_empty_input_returns_base() {
        let base = create_test_dir("new_empty");
        let result = check_new_path_safe(&base, "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), base.canonicalize().unwrap());
        remove_test_dir("new_empty");
    }

    #[test]
    fn new_path_accepts_nested_path() {
        let base = create_test_dir("new_nested");
        let result = check_new_path_safe(&base, "a/b/c/d/e/f");
        assert!(result.is_ok());
        remove_test_dir("new_nested");
    }

    #[test]
    fn new_path_accepts_dot_slash_prefix() {
        let base = create_test_dir("new_dot");
        let result = check_new_path_safe(&base, "./a");
        assert!(result.is_ok());
        remove_test_dir("new_dot");
    }

    #[test]
    fn new_path_accepts_dot_in_middle() {
        let base = create_test_dir("new_dotmid");
        let result = check_new_path_safe(&base, "a/./b");
        assert!(result.is_ok());
        remove_test_dir("new_dotmid");
    }

    #[test]
    fn new_path_accepts_up_and_back_down() {
        let base = create_test_dir("new_updown");
        // a/../b  =  b  within base
        let result = check_new_path_safe(&base, "a/../b");
        assert!(result.is_ok());
        remove_test_dir("new_updown");
    }

    #[test]
    fn new_path_goes_up_exactly_to_base() {
        let base = create_test_dir("new_upbase");
        let result = check_new_path_safe(&base, "a/b/../..");
        assert!(result.is_ok());
        remove_test_dir("new_upbase");
    }

    #[test]
    fn new_path_accepts_existing_file() {
        let base = create_test_dir("new_existing");
        fs::File::create(base.join("existing.txt")).unwrap();
        let result = check_new_path_safe(&base, "existing.txt");
        assert!(result.is_ok());
        remove_test_dir("new_existing");
    }

    #[test]
    fn new_path_rejects_dotdot() {
        let base = create_test_dir("new_dotdot");
        let result = check_new_path_safe(&base, "..");
        assert!(result.is_err());
        remove_test_dir("new_dotdot");
    }

    #[test]
    fn new_path_rejects_deep_traversal_with_normal_prefix() {
        let base = create_test_dir("new_deep");
        let result = check_new_path_safe(&base, "a/../../..");
        assert!(result.is_err());
        remove_test_dir("new_deep");
    }

    #[test]
    fn new_path_rejects_mid_traversal_before_resolving() {
        let base = create_test_dir("new_mid");
        // a/../..  goes above base at the second .. , before reaching "b"
        let result = check_new_path_safe(&base, "a/../../b");
        assert!(result.is_err());
        remove_test_dir("new_mid");
    }

    #[test]
    fn new_path_root_base_rejects_absolute_input() {
        let base = PathBuf::from("/");
        let result = check_new_path_safe(base, "/etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn new_path_non_root_base_rejects_absolute_input() {
        let base = create_test_dir("new_abs");
        let result = check_new_path_safe(&base, "/etc/passwd");
        assert!(result.is_err());
        remove_test_dir("new_abs");
    }

    #[test]
    fn new_path_base_must_exist() {
        let base = PathBuf::from("/tmp/security_test/new_nonexistent_base");
        let _ = fs::remove_dir_all(&base);
        let result = check_new_path_safe(&base, "file.txt");
        assert!(result.is_err());
    }

    // ── check_existing_path_safe ────────────────────────────────────

    #[test]
    fn existing_path_accepts_valid_file() {
        let base = create_test_dir("exist_valid");
        fs::File::create(base.join("existing.txt")).unwrap();
        let result = check_existing_path_safe(&base, "existing.txt");
        assert!(result.is_ok());
        remove_test_dir("exist_valid");
    }

    #[test]
    fn existing_path_rejects_non_existent() {
        let base = create_test_dir("exist_noexist");
        let result = check_existing_path_safe(&base, "missing.txt");
        assert!(result.is_err());
        remove_test_dir("exist_noexist");
    }

    #[test]
    fn existing_path_rejects_dotdot() {
        let base = create_test_dir("exist_dotdot");
        let result = check_existing_path_safe(&base, "..");
        assert!(result.is_err());
        remove_test_dir("exist_dotdot");
    }

    #[test]
    fn existing_path_rejects_traversal() {
        let base = create_test_dir("exist_trav");
        let result = check_existing_path_safe(&base, "../../");
        assert!(result.is_err());
        remove_test_dir("exist_trav");
    }

    #[test]
    fn existing_path_rejects_null_byte() {
        let base = create_test_dir("exist_null");
        let result = check_existing_path_safe(&base, "file\0.txt");
        assert!(result.is_err());
        remove_test_dir("exist_null");
    }

    #[test]
    fn existing_path_rejects_absolute_input() {
        let base = create_test_dir("exist_abs");
        let result = check_existing_path_safe(&base, "/etc/passwd");
        assert!(result.is_err());
        remove_test_dir("exist_abs");
    }

    #[test]
    fn existing_path_non_existent_absolute_input() {
        let base = create_test_dir("exist_abs_noexist");
        // Absolute input is rejected at the RootDir check, before canonicalize.
        // This means it's rejected even if the target file doesn't exist.
        let result = check_existing_path_safe(&base, "/nonexistent/path");
        assert!(result.is_err());
        remove_test_dir("exist_abs_noexist");
    }

    #[test]
    fn existing_path_base_must_exist() {
        let base = PathBuf::from("/tmp/security_test/exist_no_base");
        let _ = fs::remove_dir_all(&base);
        let result = check_existing_path_safe(&base, "file.txt");
        assert!(result.is_err());
    }
}
