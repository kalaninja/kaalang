//! Replaces a finished diagram without damaging its sources or the previous export.

use std::{
    fs::{self, OpenOptions},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
};

use tempfile::{Builder, NamedTempFile};

pub(super) fn write<'a>(
    output: &Path,
    svg: &str,
    sources: impl Iterator<Item = &'a Path>,
) -> io::Result<()> {
    let path = destination(output)?;
    // Opening without truncating also checks the existing file's write permissions.
    let existing = match OpenOptions::new().write(true).open(&path) {
        Ok(mut file) => {
            let metadata = file.metadata()?;
            if !metadata.is_file() {
                return file.write_all(svg.as_bytes());
            }
            #[cfg(unix)]
            let is_output = {
                use std::os::unix::fs::MetadataExt;
                let id = (metadata.dev(), metadata.ino());
                move |path: &Path| fs::metadata(path).map(|other| (other.dev(), other.ino()) == id)
            };
            #[cfg(not(unix))]
            let is_output = {
                let output = same_file::Handle::from_file(file.try_clone()?)?;
                move |path: &Path| source_handle(path).map(|other| other == output)
            };
            for source in sources {
                match is_output(source) {
                    Ok(true) => {
                        return Err(io::Error::other(
                            "the output path would overwrite the source file",
                        ));
                    }
                    // The module walk already reported any inaccessible source.
                    Err(error)
                        if !matches!(
                            error.kind(),
                            ErrorKind::NotFound | ErrorKind::PermissionDenied
                        ) =>
                    {
                        return Err(error);
                    }
                    _ => {}
                }
            }
            Some(file)
        }
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let mut file = create_sibling(&path, existing.is_some())?;
    // The replacement keeps the old file's mode, but not its owner or ACL.
    if let Some(existing) = &existing {
        file.as_file()
            .set_permissions(existing.metadata()?.permissions())?;
    }
    file.write_all(svg.as_bytes())?;
    file.as_file().sync_all()?;
    publish(file, &path, existing.is_some())
}

fn publish(temporary: NamedTempFile, path: &Path, replacing: bool) -> io::Result<()> {
    let temporary = temporary.into_temp_path();
    if replacing {
        temporary.persist(path).map_err(|err| err.error)
    } else {
        // A file created since our lookup has its own contents and access controls.
        temporary.persist_noclobber(path).map_err(|err| err.error)
    }
}

#[cfg(windows)]
fn source_handle(path: &Path) -> io::Result<same_file::Handle> {
    use std::os::windows::fs::OpenOptionsExt;
    // Windows permits metadata queries without read access, including on directories.
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    let file = OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    same_file::Handle::from_file(file)
}

#[cfg(not(any(unix, windows)))]
fn source_handle(path: &Path) -> io::Result<same_file::Handle> {
    same_file::Handle::from_path(path)
}

/// A successful export through a symlink keeps the link, even if its target is new.
fn destination(output: &Path) -> io::Result<PathBuf> {
    match fs::metadata(output) {
        Ok(metadata) if metadata.is_file() => output.canonicalize(),
        Ok(_) => Ok(output.to_owned()),
        Err(error) if error.kind() == ErrorKind::NotFound => match fs::read_link(output) {
            Ok(link) => destination(&output.with_file_name(link)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(output.to_owned()),
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    }
}

fn create_sibling(path: &Path, replacing: bool) -> io::Result<NamedTempFile> {
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut builder = Builder::new();
    builder.prefix(".kaalang-").suffix(".tmp");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if replacing {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // The temporary path can be unlinked on error while its file handle is open.
        const FILE_SHARE_DELETE: u32 = 4;
        options.share_mode(FILE_SHARE_DELETE);
    }
    #[cfg(not(unix))]
    let _ = replacing;
    builder.make_in(directory, |path| options.open(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_export_keeps_a_destination_created_after_lookup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("diagram.svg");
        let mut file = create_sibling(&path, false).unwrap();
        file.write_all(b"first export").unwrap();
        fs::write(&path, b"another export").unwrap();
        assert_eq!(
            publish(file, &path, false).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&path).unwrap(), b"another export");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
