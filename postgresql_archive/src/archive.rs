//! Manage PostgreSQL archive
#![allow(dead_code)]
use crate::error::ArchiveError::{AssetHashNotFound, AssetNotFound, ReleaseNotFound, Unexpected};
use crate::error::Result;
use crate::github::{Asset, Release};
use crate::version::Version;
use bytes::Bytes;
use flate2::bufread::GzDecoder;
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::{header, RequestBuilder};
use std::fs::{create_dir_all, File};
use std::io::{copy, BufReader, Cursor};
use std::path::Path;
use std::str::FromStr;
use tar::Archive;

const GITHUB_API_VERSION_HEADER: &str = "X-GitHub-Api-Version";
const GITHUB_API_VERSION: &str = "2022-11-28";

lazy_static! {
    static ref GITHUB_TOKEN: Option<String> = match std::env::var("GITHUB_TOKEN") {
        Ok(token) => Some(token),
        Err(_) => None,
    };
}

lazy_static! {
    static ref USER_AGENT: String = format!(
        "{PACKAGE}/{VERSION}",
        PACKAGE = env!("CARGO_PKG_NAME"),
        VERSION = env!("CARGO_PKG_VERSION")
    );
}

/// Adds GitHub headers to the request builder.
trait GitHubHeaders {
    /// Adds GitHub headers to the request builder. If a GitHub token is set, then it is added as a
    /// bearer token. This is used to authenticate with the GitHub API to increase the rate limit.
    fn add_github_headers(self) -> anyhow::Result<RequestBuilder>;
}

/// Implementation that adds GitHub headers to a request builder.
impl GitHubHeaders for RequestBuilder {
    /// Adds GitHub headers to the request builder. If a GitHub token is set, then it is added as a
    /// bearer token. This is used to authenticate with the GitHub API to increase the rate limit.
    fn add_github_headers(self) -> anyhow::Result<RequestBuilder> {
        let mut headers = HeaderMap::new();

        headers.append(GITHUB_API_VERSION_HEADER, GITHUB_API_VERSION.parse()?);
        headers.append(header::USER_AGENT, USER_AGENT.parse()?);

        if let Some(token) = &*GITHUB_TOKEN {
            headers.append(header::AUTHORIZATION, format!("Bearer {token}").parse()?);
        }

        Ok(self.headers(headers))
    }
}

/// Gets a release from GitHub for a given [`version`](Version) of PostgreSQL. If a release for the
/// [`version`](Version) is not found, then a [`ReleaseNotFound`] error is returned.
async fn get_release(version: &Version) -> Result<Release> {
    let url = "https://api.github.com/repos/theseus-rs/postgresql-binaries/releases";
    let client = reqwest::Client::new();

    if version.minor.is_some() && version.release.is_some() {
        let request = client
            .get(format!("{url}/tags/{version}"))
            .add_github_headers()?;
        let response = request.send().await?.error_for_status()?;
        let release = response.json::<Release>().await?;

        return Ok(release);
    }

    let mut result: Option<Release> = None;
    let mut page = 1;

    loop {
        let request = client
            .get(url)
            .add_github_headers()?
            .query(&[("page", page.to_string().as_str()), ("per_page", "100")]);
        let response = request.send().await?.error_for_status()?;
        let response_releases = response.json::<Vec<Release>>().await?;
        if response_releases.is_empty() {
            break;
        }

        for release in response_releases {
            let release_version = Version::from_str(&release.tag_name)?;
            if version.matches(&release_version) {
                match &result {
                    Some(result_release) => {
                        let result_version = Version::from_str(&result_release.tag_name)?;
                        if release_version > result_version {
                            result = Some(release);
                        }
                    }
                    None => {
                        result = Some(release);
                    }
                }
            }
        }

        page += 1;
    }

    match result {
        Some(release) => Ok(release),
        None => Err(ReleaseNotFound(version.to_string())),
    }
}

/// Gets the version of PostgreSQL for the specified [`version`](Version).  If the version minor or release is not
/// specified, then the latest version is returned. If a release for the [`version`](Version) is not found, then a
/// [`ReleaseNotFound`] error is returned.
pub async fn get_version(version: &Version) -> Result<Version> {
    let release = get_release(version).await?;
    Version::from_str(&release.tag_name)
}

/// Gets the assets for a given [`version`](Version) of PostgreSQL and `target` (e.g. `x86_64-unknown-linux-gnu`).
/// If the [`version`](Version) or `target` is not found, then an [error](crate::error::ArchiveError) is returned.
///
/// Two assets are returned. The first [asset](Asset) is the archive, and the second [asset](Asset) is the archive hash.
async fn get_asset<S: AsRef<str>>(version: &Version, target: S) -> Result<(Version, Asset, Asset)> {
    let release = get_release(version).await?;
    let asset_version = Version::from_str(&release.tag_name)?;
    let mut asset: Option<Asset> = None;
    let mut asset_hash: Option<Asset> = None;
    let asset_name = format!("postgresql-{}-{}.tar.gz", asset_version, target.as_ref());
    let asset_hash_name = format!("{asset_name}.sha256");

    for release_asset in release.assets {
        if release_asset.name == asset_name {
            asset = Some(release_asset);
        } else if release_asset.name == asset_hash_name {
            asset_hash = Some(release_asset);
        }

        if asset.is_some() && asset_hash.is_some() {
            break;
        }
    }

    match (asset, asset_hash) {
        (Some(asset), Some(asset_hash)) => Ok((asset_version, asset, asset_hash)),
        (None, _) => Err(AssetNotFound(asset_name.to_string())),
        (_, None) => Err(AssetNotFound(asset_name.to_string())),
    }
}

/// Gets the archive for a given [`version`](Version) of PostgreSQL for the current target.
/// If the [`version`](Version) is not found for this target, then an [error](crate::error::ArchiveError) is returned.
///
/// Returns the archive bytes and the archive hash.
pub async fn get_archive(version: &Version) -> Result<(Version, Bytes, String)> {
    get_archive_for_target(version, target_triple::TARGET).await
}

/// Gets the archive for a given [`version`](Version) of PostgreSQL and `target` (e.g. `x86_64-unknown-linux-gnu`).
/// If the [`version`](Version) or `target` is not found, then an [error](crate::error::ArchiveError) is returned.
///
/// Returns the archive bytes and the archive hash.
pub async fn get_archive_for_target<S: AsRef<str>>(
    version: &Version,
    target: S,
) -> Result<(Version, Bytes, String)> {
    let (asset_version, asset, asset_hash) = get_asset(version, target).await?;
    let client = reqwest::Client::new();
    let request = client
        .get(asset_hash.browser_download_url)
        .add_github_headers()?;
    let response = request.send().await?.error_for_status()?;
    let text = response.text().await?;
    let re = Regex::new(r"[0-9a-f]{64}")?;
    let hash = match re.find(&text) {
        Some(hash) => hash.as_str().to_string(),
        None => return Err(AssetHashNotFound(asset.name)),
    };

    let asset_url = asset.browser_download_url;
    let request = client.get(asset_url).add_github_headers()?;
    let response = request.send().await?.error_for_status()?;
    let archive: Bytes = response.bytes().await?;

    Ok((asset_version, archive, hash))
}

/// Extracts the compressed tar `bytes` to the `out_dir`.
pub async fn extract(bytes: &Bytes, out_dir: &Path) -> Result<()> {
    let input = BufReader::new(Cursor::new(bytes));
    let decoder = GzDecoder::new(input);
    let mut archive = Archive::new(decoder);

    for file in archive.entries()? {
        let mut file_entry = file?;
        let file_header = file_entry.header();
        let file_size = file_header.size()?;
        #[cfg(unix)]
        let file_mode = file_header.mode()?;

        let file_header_path = file_header.path()?.to_path_buf();
        let prefix = match file_header_path.components().next() {
            Some(component) => component.as_os_str().to_str().unwrap_or_default(),
            None => {
                return Err(Unexpected(
                    "Failed to get file header path prefix".to_string(),
                ))
            }
        };
        let stripped_file_header_path = file_header_path.strip_prefix(prefix)?.to_path_buf();
        let mut file_name = out_dir.to_path_buf();
        file_name.push(stripped_file_header_path);

        if file_size == 0 || file_name.is_dir() {
            create_dir_all(&file_name)?;
        } else {
            let mut output_file = File::create(&file_name)?;
            copy(&mut file_entry, &mut output_file)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                output_file.set_permissions(std::fs::Permissions::from_mode(file_mode))?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Use a known, fully defined version to speed up test execution
    const VERSION: Version = Version::new(16, Some(1), Some(0));
    const INVALID_VERSION: Version = Version::new(1, Some(0), Some(0));

    #[tokio::test]
    async fn test_get_release() -> Result<()> {
        let _ = get_release(&VERSION).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_get_release_version_not_found() -> Result<()> {
        let release = get_release(&INVALID_VERSION).await;
        assert!(release.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_asset() -> Result<()> {
        let target_triple = "x86_64-unknown-linux-musl".to_string();
        let (asset_version, asset, asset_hash) = get_asset(&VERSION, &target_triple).await?;
        assert!(asset_version.matches(&VERSION));
        assert!(asset.name.contains(&target_triple));
        assert!(asset_hash.name.contains(&target_triple));
        assert!(asset_hash.name.starts_with(asset.name.as_str()));
        assert!(asset_hash.name.ends_with(".sha256"));
        Ok(())
    }

    #[tokio::test]
    async fn test_get_asset_version_not_found() -> Result<()> {
        let target_triple = "x86_64-unknown-linux-musl".to_string();
        let result = get_asset(&INVALID_VERSION, &target_triple).await;
        assert!(result.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_asset_target_not_found() -> Result<()> {
        let target_triple = "wasm64-unknown-unknown".to_string();
        let result = get_asset(&VERSION, &target_triple).await;
        assert!(result.is_err());
        Ok(())
    }
}
