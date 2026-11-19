//! Replaces a finished diagram without damaging its sources or the previous export.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
};

use tempfile::{Builder, NamedTempFile};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

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
    #[cfg(target_os = "macos")]
    if existing.is_some() {
        return macos::replace(&path, svg);
    }
    let mut file = create_sibling(&path, existing.as_ref())?;
    if let Some(existing) = &existing {
        copy_permissions(existing, file.as_file())?;
    }
    file.write_all(svg.as_bytes())?;
    file.as_file().sync_all()?;
    publish(file, &path, existing.is_some())
}

fn publish(temporary: NamedTempFile, path: &Path, replacing: bool) -> io::Result<()> {
    let temporary = temporary.into_temp_path();
    if replacing {
        fs::rename(temporary, path)
    } else {
        // A file created since our lookup has its own contents and access controls.
        temporary.persist_noclobber(path).map_err(|err| err.error)
    }
}

fn copy_permissions(existing: &File, replacement: &File) -> io::Result<()> {
    let original = existing.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, fchown};
        let created = replacement.metadata()?;
        let owner = (original.uid() != created.uid()).then(|| original.uid());
        let group = (original.gid() != created.gid()).then(|| original.gid());
        if owner.is_some() || group.is_some() {
            fchown(replacement, owner, group)?;
        }
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        use rustix::{
            fs::{XattrFlags, fgetxattr, fremovexattr, fsetxattr},
            io::Errno,
        };
        const ACL: &str = "system.posix_acl_access";
        // Linux limits a single extended attribute to 64 KiB.
        let mut acl = vec![0; 65_536];
        match fgetxattr(existing, ACL, &mut acl[..]) {
            Ok(len) => fsetxattr(replacement, ACL, &acl[..len], XattrFlags::empty())?,
            // The parent may have supplied a default ACL that the old file did not have.
            Err(Errno::NODATA) => match fremovexattr(replacement, ACL) {
                Ok(()) | Err(Errno::NODATA) => {}
                Err(error) => return Err(error.into()),
            },
            Err(Errno::NOTSUP) => {}
            Err(error) => return Err(error.into()),
        }
    }
    replacement.set_permissions(original.permissions())
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

fn create_sibling(path: &Path, existing: Option<&File>) -> io::Result<NamedTempFile> {
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut builder = Builder::new();
    builder.prefix(".kaalang-").suffix(".tmp");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    if existing.is_some() {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(windows::FILE_SHARE_DELETE);
    }
    let temporary = builder.make_in(directory, |path| options.open(path))?;
    #[cfg(windows)]
    if let Some(existing) = existing {
        return windows::with_permissions(temporary, existing, &builder, directory);
    }
    #[cfg(not(any(unix, windows)))]
    let _ = existing;
    Ok(temporary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_export_keeps_a_destination_created_after_lookup() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("diagram.svg");
        let mut file = create_sibling(&path, None).unwrap();
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
