use std::{
    fs::File,
    io,
    os::windows::io::{AsHandle as _, OwnedHandle},
    path::Path,
};

use tempfile::{Builder, NamedTempFile};
use windows_namespace_request_sys::{
    OpenFile, QueryVolumeInformation, Win32Error,
    security::{
        ObjectType, SecurityAttributes, SecurityCaptureError, SecurityDescriptor,
        SecurityInformation, get_security_info,
    },
};

// The temporary path can be unlinked on error while its file handle is open.
pub(super) const FILE_SHARE_DELETE: u32 = 4;

pub(super) fn with_permissions(
    temporary: NamedTempFile,
    original: &File,
    builder: &Builder<'_, '_>,
    directory: &Path,
) -> io::Result<NamedTempFile> {
    const FILE_PERSISTENT_ACLS: u32 = 0x8;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const CREATE_NEW: u32 = 1;

    let information =
        SecurityInformation::OWNER | SecurityInformation::GROUP | SecurityInformation::DACL;
    let read = |file: &File| {
        get_security_info(file.as_handle(), ObjectType::FILE, information)
            .map_err(SecurityCaptureError::into_source)
    };
    let original = match read(original) {
        Ok(descriptor) => descriptor,
        Err(err) => {
            let volume = QueryVolumeInformation::new(OwnedHandle::from(original.try_clone()?))
                .perform()
                .map_err(Win32Error::to_io_error)?;
            if volume.flags() & FILE_PERSISTENT_ACLS == 0 {
                return Ok(temporary);
            }
            return Err(err);
        }
    };
    let matches = |file: &File| -> io::Result<bool> {
        let actual = read(file)?;
        let sddl = |descriptor: &SecurityDescriptor| {
            descriptor
                .to_sddl(information)
                .map_err(SecurityCaptureError::into_source)
        };
        Ok(original == actual || sddl(&original)? == sddl(&actual)?)
    };
    if matches(temporary.as_file())? {
        return Ok(temporary);
    }
    temporary.into_temp_path().close()?;
    // Create with the source's security descriptor. Windows can still apply the
    // parent's inheritable entries, so check the actual ACL before writing any data.
    let temporary = builder.make_in(directory, |path| {
        let descriptor = original
            .try_clone()
            .map_err(SecurityCaptureError::into_source)?;
        let handle = OpenFile::new(path.as_os_str().into())
            .map_err(io::Error::other)?
            .with_security(Some(SecurityAttributes::new(Some(descriptor), false)))
            .with_desired_access(GENERIC_WRITE)
            .with_share_mode(FILE_SHARE_DELETE)
            .with_creation_disposition(CREATE_NEW)
            .perform()
            .map_err(Win32Error::to_io_error)?;
        Ok(File::from(handle))
    })?;
    if !matches(temporary.as_file())? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "could not preserve the output file's access controls",
        ));
    }
    Ok(temporary)
}
