use std::fs::File;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use crate::errors::*;
use crate::iter::Iter;

pub struct Finder<'a> {
    filename: &'a Path,
}

impl<'a> Finder<'a> {
    pub fn new() -> Self {
        Finder {
            filename: Path::new(".env"),
        }
    }

    pub fn filename(mut self, filename: &'a Path) -> Self {
        self.filename = filename;
        self
    }

    pub fn find(self) -> Result<Iter<File>> {
        let current_dir = env::current_dir().map_err(|source| IoError::without_path(source))?;
        let path = find(&current_dir, self.filename)?;
        let file = File::open(&path).map_err(|source| IoError::from_parts(path.clone().into(), source))?;
        let iter = Iter::new(path.into(), file);
        Ok(iter)
    }
}

/// Searches for `filename` in `directory` and parent directories until found or root is reached.
pub fn find(directory: &Path, filename: &Path) -> Result<PathBuf> {
    let candidate = directory.join(filename);

    match fs::metadata(&candidate) {
        Ok(metadata) => {
            if metadata.is_file() {
                return Ok(candidate);
            }
        }
        Err(error) => {
            if error.kind() != io::ErrorKind::NotFound {
                return Err(IoError::from_parts(candidate.into(), error).into());
            }
        }
    }

    if let Some(parent) = directory.parent() {
        find(parent, filename)
    } else {
        let source = io::Error::new(io::ErrorKind::NotFound, "path not found");
        Err(IoError::from_parts(directory.into(), source).into())
    }
}
