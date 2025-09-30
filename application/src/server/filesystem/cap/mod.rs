use cap_std::fs::{Metadata, OpenOptions, PermissionsExt};
use std::{
    collections::VecDeque,
    os::{fd::AsFd, unix::fs::PermissionsExt as StdPermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
pub use utils::{AsyncReadDir, AsyncWalkDir, ReadDir, WalkDir};

mod utils;

#[derive(Debug, Clone)]
pub struct CapFilesystem {
    pub base_path: Arc<PathBuf>,
    pub(super) inner: Arc<RwLock<Option<Arc<cap_std::fs::Dir>>>>,
}

impl CapFilesystem {
    pub async fn new(base_path: PathBuf) -> Result<Self, std::io::Error> {
        let inner = tokio::task::spawn_blocking({
            let base_path = base_path.clone();
            move || cap_std::fs::Dir::open_ambient_dir(&base_path, cap_std::ambient_authority())
        })
        .await??;

        Ok(Self {
            base_path: Arc::new(base_path),
            inner: Arc::new(RwLock::new(Some(Arc::new(inner)))),
        })
    }

    pub fn new_uninitialized(base_path: PathBuf) -> Self {
        Self {
            base_path: Arc::new(base_path),
            inner: Arc::new(RwLock::new(None)),
        }
    }

    #[inline]
    pub async fn is_uninitialized(&self) -> bool {
        self.inner.read().await.is_none()
    }

    #[inline]
    pub async fn async_get_inner(&self) -> Result<Arc<cap_std::fs::Dir>, anyhow::Error> {
        let inner = self.inner.read().await;

        inner
            .clone()
            .ok_or_else(|| anyhow::anyhow!("filesystem not initialized"))
    }

    #[inline]
    pub fn get_inner(&self) -> Result<Arc<cap_std::fs::Dir>, anyhow::Error> {
        let inner = self.inner.blocking_read();

        inner
            .clone()
            .ok_or_else(|| anyhow::anyhow!("filesystem not initialized"))
    }

    #[inline]
    pub fn resolve_path(path: &Path) -> PathBuf {
        let mut result = PathBuf::new();

        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    if !result.as_os_str().is_empty()
                        && result.components().next_back() != Some(std::path::Component::RootDir)
                    {
                        result.pop();
                    }
                }
                _ => {
                    result.push(component);
                }
            }
        }

        result
    }

    #[inline]
    pub fn relative_path(&self, path: &Path) -> PathBuf {
        Self::resolve_path(if let Ok(path) = path.strip_prefix(&*self.base_path) {
            path
        } else if let Ok(path) = path.strip_prefix("/") {
            path
        } else {
            path
        })
    }

    pub async fn async_create_dir_all(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.create_dir_all(path)).await??;

        Ok(())
    }

    pub fn create_dir_all(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        inner.create_dir_all(path)?;

        Ok(())
    }

    pub async fn async_create_dir(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.create_dir(path)).await??;

        Ok(())
    }

    pub fn create_dir(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        inner.create_dir(path)?;

        Ok(())
    }

    pub async fn async_remove_dir(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.remove_dir(path)).await??;

        Ok(())
    }

    pub fn remove_dir(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        inner.remove_dir(path)?;

        Ok(())
    }

    pub async fn async_remove_dir_all(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.remove_dir_all(path)).await??;

        Ok(())
    }

    pub fn remove_dir_all(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        inner.remove_dir_all(path)?;

        Ok(())
    }

    pub async fn async_remove_file(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.remove_file(path)).await??;

        Ok(())
    }

    pub fn remove_file(&self, path: impl AsRef<Path>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        inner.remove_file(path)?;

        Ok(())
    }

    pub async fn async_rename(
        &self,
        from: impl AsRef<Path>,
        to_dir: &CapFilesystem,
        to: impl AsRef<Path>,
    ) -> Result<(), anyhow::Error> {
        let from = self.relative_path(from.as_ref());
        let to = self.relative_path(to.as_ref());

        let inner = self.async_get_inner().await?;
        let to_inner = to_dir.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.rename(from, &to_inner, to)).await??;

        Ok(())
    }

    pub fn rename(
        &self,
        from: impl AsRef<Path>,
        to_dir: &CapFilesystem,
        to: impl AsRef<Path>,
    ) -> Result<(), anyhow::Error> {
        let from = self.relative_path(from.as_ref());
        let to = self.relative_path(to.as_ref());

        let inner = self.get_inner()?;
        let to_inner = to_dir.get_inner()?;
        inner.rename(from, &to_inner, to)?;

        Ok(())
    }

    pub async fn async_metadata(&self, path: impl AsRef<Path>) -> Result<Metadata, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let metadata = if path.components().next().is_none() {
            cap_std::fs::Metadata::from_just_metadata(tokio::fs::metadata(&*self.base_path).await?)
        } else {
            let inner = self.async_get_inner().await?;

            tokio::task::spawn_blocking(move || inner.metadata(path)).await??
        };

        Ok(metadata)
    }

    pub fn metadata(&self, path: impl AsRef<Path>) -> Result<Metadata, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let metadata = if path.components().next().is_none() {
            cap_std::fs::Metadata::from_just_metadata(std::fs::metadata(&*self.base_path)?)
        } else {
            let inner = self.get_inner()?;

            inner.metadata(path)?
        };

        Ok(metadata)
    }

    pub async fn async_symlink_metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Metadata, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let metadata = if path.components().next().is_none() {
            cap_std::fs::Metadata::from_just_metadata(
                tokio::fs::symlink_metadata(&*self.base_path).await?,
            )
        } else {
            let inner = self.async_get_inner().await?;

            tokio::task::spawn_blocking(move || inner.symlink_metadata(path)).await??
        };

        Ok(metadata)
    }

    pub fn symlink_metadata(&self, path: impl AsRef<Path>) -> Result<Metadata, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let metadata = if path.components().next().is_none() {
            cap_std::fs::Metadata::from_just_metadata(std::fs::symlink_metadata(&*self.base_path)?)
        } else {
            let inner = self.get_inner()?;

            inner.symlink_metadata(path)?
        };

        Ok(metadata)
    }

    pub async fn async_canonicalize(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, anyhow::Error> {
        let path = self.relative_path(path.as_ref());
        if path.components().next().is_none() {
            return Ok(path);
        }

        let inner = self.async_get_inner().await?;
        let canonicalized = tokio::task::spawn_blocking(move || inner.canonicalize(path)).await??;

        Ok(canonicalized)
    }

    pub fn canonicalize(&self, path: impl AsRef<Path>) -> Result<PathBuf, anyhow::Error> {
        let path = self.relative_path(path.as_ref());
        if path.components().next().is_none() {
            return Ok(path);
        }

        let inner = self.get_inner()?;
        let canonicalized = inner.canonicalize(path)?;

        Ok(canonicalized)
    }

    pub async fn async_read_link(&self, path: impl AsRef<Path>) -> Result<PathBuf, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        let link = tokio::task::spawn_blocking(move || inner.read_link(path)).await??;

        Ok(link)
    }

    pub fn read_link(&self, path: impl AsRef<Path>) -> Result<PathBuf, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        let link = inner.read_link(path)?;

        Ok(link)
    }

    pub async fn async_read_link_contents(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        let link_contents =
            tokio::task::spawn_blocking(move || inner.read_link_contents(path)).await??;

        Ok(link_contents)
    }

    pub fn read_link_contents(&self, path: impl AsRef<Path>) -> Result<PathBuf, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        let link_contents = inner.read_link_contents(path)?;

        Ok(link_contents)
    }

    pub async fn async_read_to_string(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<String, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        let content = tokio::task::spawn_blocking(move || inner.read_to_string(path)).await??;

        Ok(content)
    }

    pub fn read_to_string(&self, path: impl AsRef<Path>) -> Result<String, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        let content = inner.read_to_string(path)?;

        Ok(content)
    }

    pub async fn async_open(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<tokio::fs::File, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        let file = tokio::task::spawn_blocking(move || inner.open(path)).await??;

        Ok(tokio::fs::File::from_std(file.into_std()))
    }

    pub fn open(&self, path: impl AsRef<Path>) -> Result<std::fs::File, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        let file = inner.open(path)?;

        Ok(file.into_std())
    }

    pub async fn async_open_with(
        &self,
        path: impl AsRef<Path>,
        options: OpenOptions,
    ) -> Result<tokio::fs::File, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        let file = tokio::task::spawn_blocking(move || inner.open_with(path, &options)).await??;

        Ok(tokio::fs::File::from_std(file.into_std()))
    }

    pub fn open_with(
        &self,
        path: impl AsRef<Path>,
        options: OpenOptions,
    ) -> Result<std::fs::File, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        let file = inner.open_with(path, &options)?;

        Ok(file.into_std())
    }

    pub async fn async_write(
        &self,
        path: impl AsRef<Path>,
        data: Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.write(path, data)).await??;

        Ok(())
    }

    pub fn write(&self, path: impl AsRef<Path>, data: Vec<u8>) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        inner.write(path, data)?;

        Ok(())
    }

    pub async fn async_create(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<tokio::fs::File, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.async_get_inner().await?;
        let file = tokio::task::spawn_blocking(move || inner.create(path)).await??;

        Ok(tokio::fs::File::from_std(file.into_std()))
    }

    pub fn create(&self, path: impl AsRef<Path>) -> Result<std::fs::File, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        let file = inner.create(path)?;

        Ok(file.into_std())
    }

    pub async fn async_copy(
        &self,
        from: impl AsRef<Path>,
        to_dir: &CapFilesystem,
        to: impl AsRef<Path>,
    ) -> Result<u64, anyhow::Error> {
        let from = self.relative_path(from.as_ref());
        let to = self.relative_path(to.as_ref());

        let inner = self.async_get_inner().await?;
        let to_inner = to_dir.async_get_inner().await?;
        let bytes_copied =
            tokio::task::spawn_blocking(move || inner.copy(from, &to_inner, to)).await??;

        Ok(bytes_copied)
    }

    pub fn copy(
        &self,
        from: impl AsRef<Path>,
        to_dir: &CapFilesystem,
        to: impl AsRef<Path>,
    ) -> Result<u64, anyhow::Error> {
        let from = self.relative_path(from.as_ref());
        let to = self.relative_path(to.as_ref());

        let inner = self.get_inner()?;
        let to_inner = to_dir.get_inner()?;
        let bytes_copied = inner.copy(from, &to_inner, to)?;

        Ok(bytes_copied)
    }

    pub async fn async_set_permissions(
        &self,
        path: impl AsRef<Path>,
        permissions: cap_std::fs::Permissions,
    ) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        if path.components().next().is_none() {
            tokio::fs::set_permissions(
                &*self.base_path,
                std::fs::Permissions::from_mode(permissions.mode()),
            )
            .await?;
        } else {
            let inner = self.async_get_inner().await?;

            tokio::task::spawn_blocking(move || inner.set_permissions(path, permissions)).await??;
        }

        Ok(())
    }

    pub fn set_permissions(
        &self,
        path: impl AsRef<Path>,
        permissions: cap_std::fs::Permissions,
    ) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        let inner = self.get_inner()?;
        inner.set_permissions(path, permissions)?;

        Ok(())
    }

    pub async fn async_set_symlink_permissions(
        &self,
        path: impl AsRef<Path>,
        permissions: cap_std::fs::Permissions,
    ) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        if path.components().next().is_none() {
            tokio::fs::set_permissions(
                &*self.base_path,
                std::fs::Permissions::from_mode(permissions.mode()),
            )
            .await?;
        } else {
            let inner = self.async_get_inner().await?;

            tokio::task::spawn_blocking(move || {
                rustix::fs::chmodat(
                    inner.as_fd(),
                    path,
                    rustix::fs::Mode::from_raw_mode(permissions.mode()),
                    rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
                )
            })
            .await??;
        }

        Ok(())
    }

    pub fn set_symlink_permissions(
        &self,
        path: impl AsRef<Path>,
        permissions: cap_std::fs::Permissions,
    ) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        if path.components().next().is_none() {
            std::fs::set_permissions(
                &*self.base_path,
                std::fs::Permissions::from_mode(permissions.mode()),
            )?;
        } else {
            let inner = self.get_inner()?;

            rustix::fs::chmodat(
                inner.as_fd(),
                path,
                rustix::fs::Mode::from_raw_mode(permissions.mode()),
                rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
            )?;
        }

        Ok(())
    }

    pub async fn async_set_times(
        &self,
        path: impl AsRef<Path>,
        modification_time: std::time::SystemTime,
        access_time: Option<std::time::SystemTime>,
    ) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());
        let inner = self.async_get_inner().await?;

        let elapsed_modification = modification_time.duration_since(std::time::UNIX_EPOCH)?;
        let elapsed_access = access_time
            .unwrap_or_else(std::time::SystemTime::now)
            .duration_since(std::time::UNIX_EPOCH)?;

        let times = rustix::fs::Timestamps {
            last_modification: elapsed_modification.try_into()?,
            last_access: elapsed_access.try_into()?,
        };

        tokio::task::spawn_blocking(move || {
            rustix::fs::utimensat(
                inner.as_fd(),
                path,
                &times,
                rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
            )
        })
        .await??;

        Ok(())
    }

    pub fn set_times(
        &self,
        path: impl AsRef<Path>,
        modification_time: std::time::SystemTime,
        access_time: Option<std::time::SystemTime>,
    ) -> Result<(), anyhow::Error> {
        let path = self.relative_path(path.as_ref());
        let inner = self.get_inner()?;

        let elapsed_modification = modification_time.duration_since(std::time::UNIX_EPOCH)?;
        let elapsed_access = access_time
            .unwrap_or_else(std::time::SystemTime::now)
            .duration_since(std::time::UNIX_EPOCH)?;

        let times = rustix::fs::Timestamps {
            last_modification: elapsed_modification.try_into()?,
            last_access: elapsed_access.try_into()?,
        };

        rustix::fs::utimensat(
            inner.as_fd(),
            path,
            &times,
            rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
        )?;

        Ok(())
    }

    pub async fn async_symlink(
        &self,
        target: impl AsRef<Path>,
        link: impl AsRef<Path>,
    ) -> Result<(), anyhow::Error> {
        let target = self.relative_path(target.as_ref());
        let link = self.relative_path(link.as_ref());

        let inner = self.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.symlink(target, link)).await??;

        Ok(())
    }

    pub fn symlink(
        &self,
        target: impl AsRef<Path>,
        link: impl AsRef<Path>,
    ) -> Result<(), anyhow::Error> {
        let target = self.relative_path(target.as_ref());
        let link = self.relative_path(link.as_ref());

        let inner = self.get_inner()?;
        inner.symlink(target, link)?;

        Ok(())
    }

    pub async fn async_hard_link(
        &self,
        target: impl AsRef<Path>,
        dst_dir: &CapFilesystem,
        link: impl AsRef<Path>,
    ) -> Result<(), anyhow::Error> {
        let target = self.relative_path(target.as_ref());
        let link = self.relative_path(link.as_ref());

        let inner = self.async_get_inner().await?;
        let dst_inner = dst_dir.async_get_inner().await?;
        tokio::task::spawn_blocking(move || inner.hard_link(target, &dst_inner, link)).await??;

        Ok(())
    }

    pub fn hard_link(
        &self,
        target: impl AsRef<Path>,
        dst_dir: &CapFilesystem,
        link: impl AsRef<Path>,
    ) -> Result<(), anyhow::Error> {
        let target = self.relative_path(target.as_ref());
        let link = self.relative_path(link.as_ref());

        let inner = self.get_inner()?;
        let dst_inner = dst_dir.get_inner()?;
        inner.hard_link(target, &dst_inner, link)?;

        Ok(())
    }

    pub async fn async_read_dir_all(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<String>, anyhow::Error> {
        let mut read_dir = self.async_read_dir(path).await?;

        let mut names = Vec::new();
        while let Some(Ok((_, entry))) = read_dir.next_entry().await {
            names.push(entry);
        }

        Ok(names)
    }

    pub fn read_dir_all(&self, path: impl AsRef<Path>) -> Result<Vec<String>, anyhow::Error> {
        let mut read_dir = self.read_dir(path)?;

        let mut names = Vec::new();
        while let Some(Ok((_, entry))) = read_dir.next_entry() {
            names.push(entry);
        }

        Ok(names)
    }

    pub async fn async_read_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AsyncReadDir, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        Ok(if path.components().next().is_none() {
            AsyncReadDir::Tokio(utils::AsyncTokioReadDir(
                tokio::fs::read_dir(&*self.base_path).await?,
            ))
        } else {
            let inner = self.async_get_inner().await?;

            AsyncReadDir::Cap(utils::AsyncCapReadDir(
                Some(tokio::task::spawn_blocking(move || inner.read_dir(path)).await??),
                Some(VecDeque::with_capacity(32)),
            ))
        })
    }

    pub fn read_dir(&self, path: impl AsRef<Path>) -> Result<ReadDir, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        Ok(if path.components().next().is_none() {
            ReadDir::Std(utils::StdReadDir(std::fs::read_dir(&*self.base_path)?))
        } else {
            let inner = self.get_inner()?;

            ReadDir::Cap(utils::CapReadDir(inner.read_dir(path)?))
        })
    }

    pub async fn async_walk_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<AsyncWalkDir<'_>, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        AsyncWalkDir::new(self.clone(), path).await
    }

    pub fn walk_dir(&self, path: impl AsRef<Path>) -> Result<WalkDir<'_>, anyhow::Error> {
        let path = self.relative_path(path.as_ref());

        WalkDir::new(self.clone(), path)
    }
}
