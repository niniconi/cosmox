use std::{
    fs::{self, DirEntry, Metadata},
    io,
    path::{Path, PathBuf},
};

pub fn is_hide<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let path = path.as_ref();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        let metadata = path.metadata()?;
        let attributes = metadata.file_attributes();
        // FILE_ATTRIBUTE_HIDDEN == 0x02
        Ok((attributes & 0x2) != 0)
    }

    #[cfg(unix)]
    {
        if let Some(file_name) = path.file_name()
            && let Some(name_str) = file_name.to_str()
        {
            Ok(name_str.starts_with('.') && name_str != "." && name_str != "..")
        } else {
            Ok(false)
        }
    }

    #[cfg(not(any(windows, unix)))]
    {
        Ok(false)
    }
}

pub fn walk_dir<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    let mut dirs = vec![PathBuf::from(path.as_ref())];
    let mut result = Vec::with_capacity(16);
    while !dirs.is_empty()
        && let Some(path) = dirs.pop()
    {
        let entries = fs::read_dir(path)?;

        for entry in entries {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
            {
                if metadata.is_dir() {
                    dirs.push(entry.path());
                }
                result.push(entry.path());
            }
        }
    }
    Ok(result)
}

pub fn walk_dir_filter<P, F>(path: P, filter: F) -> io::Result<Vec<PathBuf>>
where
    P: AsRef<Path>,
    F: Fn(&Metadata, &DirEntry) -> bool,
{
    let mut dirs = vec![PathBuf::from(path.as_ref())];
    let mut result = Vec::with_capacity(16);
    while !dirs.is_empty()
        && let Some(path) = dirs.pop()
    {
        let entries = fs::read_dir(path)?;

        for entry in entries {
            if let Ok(entry) = entry
                && let Ok(metadata) = entry.metadata()
            {
                if filter(&metadata, &entry) {
                    result.push(entry.path());
                }
                if metadata.is_dir() {
                    dirs.push(entry.path());
                }
            }
        }
    }
    Ok(result)
}

#[inline]
pub fn walk_dir_with_ext<P: AsRef<Path>>(path: P, ext: &'static str) -> io::Result<Vec<PathBuf>> {
    let ext = &ext.replace(".", "");
    walk_dir_filter(path, |metadata, entry| {
        let path = &entry.path();
        let file_ext = match path.extension() {
            Some(os_str) => os_str.to_str(),
            None => None,
        };
        metadata.is_file() && file_ext.is_some_and(|x| x == ext)
    })
}

#[inline]
pub fn walk_dir_files<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    walk_dir_filter(path, |metadata, _| metadata.is_file())
}

enum CleanupEntry<'a> {
    File(&'a Path),
    Dir(&'a Path),
}

/// Automatically removes registered files/directories on drop unless [`disarm`]ed.
///
/// Useful for ensuring partially-written files are cleaned up when an operation
/// fails mid-way. Cleanup failures are silently ignored since the originating
/// error is already propagating.
pub struct FileCleanupGuard<'a> {
    entries: Vec<CleanupEntry<'a>>,
    disarmed: bool,
}

impl<'a> FileCleanupGuard<'a> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            disarmed: false,
        }
    }

    pub fn add_file(&mut self, path: &'a Path) {
        self.entries.push(CleanupEntry::File(path));
    }

    pub fn add_dir(&mut self, path: &'a Path) {
        self.entries.push(CleanupEntry::Dir(path));
    }

    /// Disarm the guard — registered paths will not be removed on drop.
    pub fn disarm(mut self) {
        self.disarmed = true;
    }
}

impl<'a> Default for FileCleanupGuard<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Drop for FileCleanupGuard<'a> {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        for entry in &self.entries {
            let _ = match entry {
                CleanupEntry::File(p) => std::fs::remove_file(p),
                CleanupEntry::Dir(p) => std::fs::remove_dir_all(p),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_guard_does_nothing_on_drop() {
        let guard = FileCleanupGuard::new();
        drop(guard);
    }

    #[test]
    fn guard_removes_file_on_drop() {
        let dir = std::env::temp_dir().join("cleanup_test_file");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let file_path = dir.join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();
        assert!(file_path.exists());

        {
            let mut guard = FileCleanupGuard::new();
            guard.add_file(&file_path);
        }

        assert!(!file_path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn disarmed_guard_keeps_file() {
        let dir = std::env::temp_dir().join("cleanup_test_disarmed");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let file_path = dir.join("test.txt");
        std::fs::write(&file_path, b"hello").unwrap();

        let mut guard = FileCleanupGuard::new();
        guard.add_file(&file_path);
        guard.disarm();

        assert!(file_path.exists());
        let _ = std::fs::remove_file(&file_path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn guard_removes_directory_on_drop() {
        let base = std::env::temp_dir().join("cleanup_test_dir");
        let _ = std::fs::remove_dir_all(&base);

        let dir_path = base.join("subdir");
        std::fs::create_dir_all(&dir_path).unwrap();
        std::fs::write(dir_path.join("a.txt"), b"a").unwrap();
        std::fs::write(dir_path.join("b.txt"), b"b").unwrap();

        assert!(dir_path.exists());

        {
            let mut guard = FileCleanupGuard::new();
            guard.add_dir(&dir_path);
        }

        assert!(
            !dir_path.exists(),
            "guard should have removed the directory tree"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn guard_removes_multiple_paths() {
        let base = std::env::temp_dir().join("cleanup_test_multi");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let f1 = base.join("f1.txt");
        let f2 = base.join("f2.txt");
        std::fs::write(&f1, b"1").unwrap();
        std::fs::write(&f2, b"2").unwrap();

        {
            let mut guard = FileCleanupGuard::new();
            guard.add_file(&f1);
            guard.add_file(&f2);
        }

        assert!(!f1.exists());
        assert!(!f2.exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
