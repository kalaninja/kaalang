use std::{
    fs::{self, File},
    io::{self, Write},
    os::unix::fs::MetadataExt as _,
    path::Path,
};

use objc2_foundation::{
    NSFileManager, NSFileManagerItemReplacementOptions, NSSearchPathDirectory,
    NSSearchPathDomainMask, NSURL,
};

/// Foundation preserves the original's access controls and provides a private
/// directory on its volume, away from the output directory's inheritable ACL.
pub(super) fn replace(path: &Path, svg: &str) -> io::Result<()> {
    let url = |path: &Path| {
        NSURL::from_file_path(path).ok_or_else(|| io::Error::other("invalid file URL"))
    };
    let original = url(path)?;
    let manager = NSFileManager::defaultManager();
    let directory = manager
        .URLForDirectory_inDomain_appropriateForURL_create_error(
            NSSearchPathDirectory::ItemReplacementDirectory,
            NSSearchPathDomainMask::UserDomainMask,
            Some(&original),
            true,
        )
        .map_err(io::Error::other)?
        .to_file_path()
        .ok_or_else(|| io::Error::other("invalid replacement directory"))?;
    let temporary = directory.join("diagram.svg");
    let result = (|| {
        let mut file = File::create_new(&temporary)?;
        let staged = file.metadata()?;
        let result = (|| {
            file.write_all(svg.as_bytes())?;
            file.sync_all()?;
            let replacement = url(&temporary)?;
            manager
                .replaceItemAtURL_withItemAtURL_backupItemName_options_resultingItemURL_error(
                    &original,
                    &replacement,
                    None,
                    NSFileManagerItemReplacementOptions::empty(),
                    None,
                )
                .map_err(io::Error::other)
        })();
        // Foundation can leave a recovery copy of the original on failure.
        if result.is_err()
            && fs::symlink_metadata(&temporary)
                .is_ok_and(|m| (m.dev(), m.ino()) == (staged.dev(), staged.ino()))
        {
            let _ = fs::remove_file(&temporary);
        }
        result
    })();
    let _ = fs::remove_dir(directory);
    result
}
