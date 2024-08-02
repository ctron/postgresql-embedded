use crate::model::AvailableExtension;
use crate::repository::portal_corp::URL;
use crate::repository::{portal_corp, Repository};
use crate::Result;
use async_trait::async_trait;
use postgresql_archive::repository::github::repository::GitHub;
use postgresql_archive::{get_archive, matcher};
use semver::{Version, VersionReq};
use std::fmt::Debug;
use std::io::Cursor;
use std::path::PathBuf;
use std::{fs, io};
use zip::ZipArchive;

/// PortalCorp repository.
#[derive(Debug)]
pub struct PortalCorp;

impl PortalCorp {
    /// Creates a new PortalCorp repository.
    ///
    /// # Errors
    /// * If the repository cannot be created
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Result<Box<dyn Repository>> {
        Ok(Box::new(Self))
    }

    /// Initializes the repository.
    ///
    /// # Errors
    /// * If the repository cannot be initialized.
    pub fn initialize() -> Result<()> {
        matcher::registry::register(|url| Ok(url.starts_with(URL)), portal_corp::matcher)?;
        postgresql_archive::repository::registry::register(
            |url| Ok(url.starts_with(URL)),
            Box::new(GitHub::new),
        )?;
        Ok(())
    }
}

#[async_trait]
impl Repository for PortalCorp {
    fn name(&self) -> &str {
        "portal-corp"
    }

    async fn get_available_extensions(&self) -> Result<Vec<AvailableExtension>> {
        let extensions = vec![AvailableExtension::new(
            self.name(),
            "pgvector_compiled",
            "Precompiled OS packages for pgvector",
        )];
        Ok(extensions)
    }

    async fn get_archive(
        &self,
        postgresql_version: &str,
        name: &str,
        version: &VersionReq,
    ) -> Result<(Version, Vec<u8>)> {
        let url = format!("{URL}/{name}?postgresql_version={postgresql_version}");
        let archive = get_archive(url.as_str(), version).await?;
        Ok(archive)
    }

    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    async fn install(
        &self,
        _name: &str,
        library_dir: PathBuf,
        extension_dir: PathBuf,
        archive: &[u8],
    ) -> Result<Vec<PathBuf>> {
        let reader = Cursor::new(archive);
        let mut archive = ZipArchive::new(reader)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Zip error"))?;
        let mut files = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Zip error"))?;
            let file_path = PathBuf::from(file.name());
            let file_path = PathBuf::from(file_path.file_name().unwrap_or_default());
            let file_name = file_path.to_string_lossy();

            if file_name.ends_with(".dylib") || file_name.ends_with(".so") {
                let mut out = Vec::new();
                io::copy(&mut file, &mut out)?;
                let path = PathBuf::from(&library_dir).join(file_path);
                fs::write(&path, out)?;
                files.push(path);
            } else if file_name.ends_with(".control") || file_name.ends_with(".sql") {
                let mut out = Vec::new();
                io::copy(&mut file, &mut out)?;
                let path = PathBuf::from(&extension_dir).join(file_path);
                fs::write(&path, out)?;
                files.push(path);
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Repository;

    #[test]
    fn test_name() {
        let repository = PortalCorp;
        assert_eq!("portal-corp", repository.name());
    }

    #[tokio::test]
    async fn test_get_available_extensions() -> Result<()> {
        let repository = PortalCorp;
        let extensions = repository.get_available_extensions().await?;
        let extension = &extensions[0];

        assert_eq!("pgvector_compiled", extension.name());
        assert_eq!(
            "Precompiled OS packages for pgvector",
            extension.description()
        );
        Ok(())
    }
}
