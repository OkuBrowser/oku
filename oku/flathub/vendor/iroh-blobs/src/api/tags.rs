//! Tags API
//!
//! The main entry point is the [`Tags`] struct.
use std::ops::RangeBounds;

use n0_future::{Stream, StreamExt};
use ref_cast::RefCast;
use tracing::trace;

pub use super::proto::{
    CreateTagRequest as CreateOptions, DeleteTagsRequest as DeleteOptions,
    ListTagsRequest as ListOptions, RenameTagRequest as RenameOptions, SetTagRequest as SetOptions,
    TagInfo,
};
use super::{
    proto::{CreateTempTagRequest, Scope},
    ApiClient, Tag, TempTag,
};
use crate::{api::proto::ListTempTagsRequest, HashAndFormat};

/// The API for interacting with tags and temp tags.
#[derive(Debug, Clone, ref_cast::RefCast)]
#[repr(transparent)]
pub struct Tags {
    client: ApiClient,
}

impl Tags {
    pub(crate) fn ref_from_sender(sender: &ApiClient) -> &Self {
        Self::ref_cast(sender)
    }

    pub async fn list_temp_tags(&self) -> irpc::Result<impl Stream<Item = HashAndFormat>> {
        let options = ListTempTagsRequest;
        trace!("{:?}", options);
        let res = self.client.rpc(options).await?;
        Ok(n0_future::stream::iter(res))
    }

    /// List all tags with options.
    ///
    /// This is the most flexible way to list tags. All the other list methods are just convenience
    /// methods that call this one with the appropriate options.
    pub async fn list_with_opts(
        &self,
        options: ListOptions,
    ) -> irpc::Result<impl Stream<Item = super::Result<TagInfo>>> {
        trace!("{:?}", options);
        let res = self.client.rpc(options).await?;
        Ok(n0_future::stream::iter(res))
    }

    /// Get the value of a single tag
    pub async fn get(&self, name: impl AsRef<[u8]>) -> super::RequestResult<Option<TagInfo>> {
        let mut stream = self
            .list_with_opts(ListOptions::single(name.as_ref()))
            .await?;
        Ok(stream.next().await.transpose()?)
    }

    pub async fn set_with_opts(&self, options: SetOptions) -> super::RequestResult<()> {
        trace!("{:?}", options);
        self.client.rpc(options).await??;
        Ok(())
    }

    pub async fn set(
        &self,
        name: impl AsRef<[u8]>,
        value: impl Into<HashAndFormat>,
    ) -> super::RequestResult<()> {
        self.set_with_opts(SetOptions {
            name: Tag::from(name.as_ref()),
            value: value.into(),
        })
        .await
    }

    /// List a range of tags
    pub async fn list_range<R, E>(
        &self,
        range: R,
    ) -> irpc::Result<impl Stream<Item = super::Result<TagInfo>>>
    where
        R: RangeBounds<E>,
        E: AsRef<[u8]>,
    {
        self.list_with_opts(ListOptions::range(range)).await
    }

    /// Lists all tags with the given prefix.
    pub async fn list_prefix(
        &self,
        prefix: impl AsRef<[u8]>,
    ) -> irpc::Result<impl Stream<Item = super::Result<TagInfo>>> {
        self.list_with_opts(ListOptions::prefix(prefix.as_ref()))
            .await
    }

    /// Lists all tags.
    pub async fn list(&self) -> irpc::Result<impl Stream<Item = super::Result<TagInfo>>> {
        self.list_with_opts(ListOptions::all()).await
    }

    /// Lists all tags with a hash_seq format.
    pub async fn list_hash_seq(&self) -> irpc::Result<impl Stream<Item = super::Result<TagInfo>>> {
        self.list_with_opts(ListOptions::hash_seq()).await
    }

    /// Deletes a tag, with full control over options. All other delete methods
    /// wrap this.
    ///
    /// Returns the number of tags actually removed. Attempting to delete a non-existent tag will *not* fail.
    pub async fn delete_with_opts(&self, options: DeleteOptions) -> super::RequestResult<u64> {
        trace!("{:?}", options);
        let deleted = self.client.rpc(options).await??;
        Ok(deleted)
    }

    /// Deletes a tag.
    ///
    /// Returns the number of tags actually removed. Attempting to delete a non-existent tag will *not* fail.
    pub async fn delete(&self, name: impl AsRef<[u8]>) -> super::RequestResult<u64> {
        self.delete_with_opts(DeleteOptions::single(name.as_ref()))
            .await
    }

    /// Deletes a range of tags.
    ///
    /// Returns the number of tags actually removed. Attempting to delete a non-existent tag will *not* fail.
    pub async fn delete_range<R, E>(&self, range: R) -> super::RequestResult<u64>
    where
        R: RangeBounds<E>,
        E: AsRef<[u8]>,
    {
        self.delete_with_opts(DeleteOptions::range(range)).await
    }

    /// Delete all tags with the given prefix.
    ///
    /// Returns the number of tags actually removed. Attempting to delete a non-existent tag will *not* fail.
    pub async fn delete_prefix(&self, prefix: impl AsRef<[u8]>) -> super::RequestResult<u64> {
        self.delete_with_opts(DeleteOptions::prefix(prefix.as_ref()))
            .await
    }

    /// Delete all tags. Use with care. After this, all data will be garbage collected.
    ///
    /// Returns the number of tags actually removed. Attempting to delete a non-existent tag will *not* fail.
    pub async fn delete_all(&self) -> super::RequestResult<u64> {
        self.delete_with_opts(DeleteOptions {
            from: None,
            to: None,
        })
        .await
    }

    /// Rename a tag atomically
    ///
    /// If the tag does not exist, this will return an error.
    pub async fn rename_with_opts(&self, options: RenameOptions) -> super::RequestResult<()> {
        trace!("{:?}", options);
        self.client.rpc(options).await??;
        Ok(())
    }

    /// Rename a tag atomically
    ///
    /// If the tag does not exist, this will return an error.
    pub async fn rename(
        &self,
        from: impl AsRef<[u8]>,
        to: impl AsRef<[u8]>,
    ) -> super::RequestResult<()> {
        self.rename_with_opts(RenameOptions {
            from: Tag::from(from.as_ref()),
            to: Tag::from(to.as_ref()),
        })
        .await
    }

    pub async fn create_with_opts(&self, options: CreateOptions) -> super::RequestResult<Tag> {
        trace!("{:?}", options);
        let rx = self.client.rpc(options);
        Ok(rx.await??)
    }

    pub async fn create(&self, value: impl Into<HashAndFormat>) -> super::RequestResult<Tag> {
        self.create_with_opts(CreateOptions {
            value: value.into(),
        })
        .await
    }

    pub async fn temp_tag(&self, value: impl Into<HashAndFormat>) -> irpc::Result<TempTag> {
        let value = value.into();
        let msg = CreateTempTagRequest {
            scope: Scope::GLOBAL,
            value,
        };
        self.client.rpc(msg).await
    }
}
