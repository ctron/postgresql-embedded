use crate::Version;
use bytes::Bytes;
use std::path::Path;
use tokio::runtime::Runtime;

lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().unwrap();
}

/// Gets the version of PostgreSQL for the specified [version](Version).  If the version minor or release is not
/// specified, then the latest version is returned. If a release for the [version](Version) is not found, then a
/// [ReleaseNotFound](crate::Error::ReleaseNotFound) error is returned.
///
/// # Errors
///
/// Returns an error if the version is not found.
pub fn get_version(releases_url: &str, version: &Version) -> crate::Result<Version> {
    RUNTIME
        .handle()
        .block_on(async move { crate::get_version(releases_url, version).await })
}

/// Gets the archive for a given [version](Version) of PostgreSQL for the current target.
/// If the [version](Version) is not found for this target, then an
/// [error](crate::Error) is returned.
///
/// Returns the archive version and bytes.
///
/// # Errors
///
/// Returns an error if the version is not found.
pub fn get_archive(releases_url: &str, version: &Version) -> crate::Result<(Version, Bytes)> {
    RUNTIME
        .handle()
        .block_on(async move { crate::get_archive(releases_url, version).await })
}

/// Gets the archive for a given [version](Version) of PostgreSQL and
/// [target](https://doc.rust-lang.org/nightly/rustc/platform-support.html).
/// If the [version](Version) or [target](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
/// is not found, then an [error](crate::error::Error) is returned.
///
/// Returns the archive version and bytes.
///
/// # Errors
///
/// Returns an error if the version or target is not found.
pub fn get_archive_for_target<S: AsRef<str>>(
    releases_url: &str,
    version: &Version,
    target: S,
) -> crate::Result<(Version, Bytes)> {
    RUNTIME
        .handle()
        .block_on(async move { crate::get_archive_for_target(releases_url, version, target).await })
}

/// Extracts the compressed tar [bytes](Bytes) to the [out_dir](Path).
///
/// # Errors
///
/// Returns an error if the extraction fails.
pub fn extract(bytes: &Bytes, out_dir: &Path) -> crate::Result<()> {
    RUNTIME
        .handle()
        .block_on(async move { crate::extract(bytes, out_dir).await })
}