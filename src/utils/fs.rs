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
