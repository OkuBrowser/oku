use super::core::{home_replica_filters, EmbeddingModality};
use crate::{database::posts::core::OkuNote, fs::OkuFs};
use bytes::Bytes;
use iroh_blobs::Hash;
use iroh_docs::AuthorId;
use iroh_docs::DocTicket;
use log::error;
use miette::IntoDiagnostic;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use std::path::PathBuf;
use url::Url;
use zebra::{
    database::default::{
        audio::DefaultAudioModel, image::DefaultImageModel, text::DefaultTextModel,
    },
    model::core::{DatabaseEmbeddingModel, DIM_BGESMALL_EN_1_5, DIM_VIT_BASE_PATCH16_224},
    Embedding,
};

impl OkuFs {
    /// The embedding vector database for text media.
    pub fn text_database(
        &self,
    ) -> miette::Result<zebra::database::default::text::DefaultTextDatabase> {
        zebra::database::default::text::DefaultTextDatabase::open_or_create(
            &"text.zebra".into(),
            &Default::default(),
        )
        .map_err(|e| miette::miette!("{e}"))
    }

    /// The embedding vector database for image media.
    pub fn image_database(
        &self,
    ) -> miette::Result<zebra::database::default::image::DefaultImageDatabase> {
        zebra::database::default::image::DefaultImageDatabase::open_or_create(
            &"image.zebra".into(),
            &Default::default(),
        )
        .map_err(|e| miette::miette!("{e}"))
    }

    /// The embedding vector database for audio media.
    pub fn audio_database(
        &self,
    ) -> miette::Result<zebra::database::default::audio::DefaultAudioDatabase> {
        zebra::database::default::audio::DefaultAudioDatabase::open_or_create(
            &"audio.zebra".into(),
            &Default::default(),
        )
        .map_err(|e| miette::miette!("{e}"))
    }

    /// Determine the modality of some data.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The given data.
    ///
    /// # Returns
    ///
    /// The modality of the data, if embeddable.
    pub fn bytes_to_embedding_modality(&self, bytes: &Bytes) -> miette::Result<EmbeddingModality> {
        let mime_type = tree_magic_mini::from_u8(bytes);
        let type_ = mime_type.split("/").nth(0).unwrap_or_default();
        match type_ {
            "audio" => Ok(EmbeddingModality::Audio),
            "image" => Ok(EmbeddingModality::Image),
            "text" => Ok(EmbeddingModality::Text),
            _ => Err(miette::miette!(
                "Unexpected MIME type ({mime_type:?}); embedding modality cannot be determined … "
            )),
        }
    }

    /// Create an embedding file in the user's home replica for a document.
    ///
    /// # Arguments
    ///
    /// * `path` - An optional path to the embedding file; if none is specified, a suggested path will be used.
    ///
    /// * `url` - The URL of the document.
    ///
    /// * `bytes` - The document's contents.
    ///
    /// # Returns
    ///
    /// The hash of the file.
    pub async fn create_post_embedding(&self, url: &Url, bytes: &Bytes) -> miette::Result<Hash> {
        let home_replica_id = self
            .home_replica()
            .await
            .ok_or(miette::miette!("No home replica set … "))?;
        let url_string = url.to_string();
        let embed_path: &PathBuf = &OkuNote::embedding_path_from_url(&url_string).into();
        let archive_path: &PathBuf = &OkuNote::archive_path_from_url(&url_string).into();
        if let Err(e) = self
            .create_or_modify_file(&home_replica_id, archive_path, bytes.clone())
            .await
        {
            error!("{e}");
        }
        match self.bytes_to_embedding_modality(bytes)? {
            EmbeddingModality::Audio => {
                let model = DefaultAudioModel::default();
                let embedding = model
                    .embed(bytes.clone())
                    .map_err(|e| miette::miette!("{e}"))?;
                let embedding_json = serde_json::to_string(&embedding).into_diagnostic()?;
                self.create_or_modify_file(&home_replica_id, embed_path, embedding_json)
                    .await
            }
            EmbeddingModality::Image => {
                let model = DefaultImageModel::default();
                let embedding = model
                    .embed(bytes.clone())
                    .map_err(|e| miette::miette!("{e}"))?;
                let embedding_json = serde_json::to_string(&embedding).into_diagnostic()?;
                self.create_or_modify_file(&home_replica_id, embed_path, embedding_json)
                    .await
            }
            EmbeddingModality::Text => {
                let model = DefaultTextModel::default();
                let embedding = model
                    .embed(bytes.clone())
                    .map_err(|e| miette::miette!("{e}"))?;
                let embedding_json = serde_json::to_string(&embedding).into_diagnostic()?;
                self.create_or_modify_file(&home_replica_id, embed_path, embedding_json)
                    .await
            }
        }
    }

    /// Find the archival records of the most similar documents.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A document.
    ///
    /// * `number_of_results` - The maximum number of archives to return.
    ///
    /// # Returns
    ///
    /// The URIs of the documents approximately most similar to the given one, paired with their archivist's authorship ID.
    pub fn nearest_archives(
        &self,
        bytes: &Bytes,
        number_of_results: usize,
    ) -> miette::Result<Vec<(AuthorId, String)>> {
        match self.bytes_to_embedding_modality(bytes)? {
            EmbeddingModality::Audio => {
                let db = self.audio_database()?;
                let results = db
                    .query_documents(&[bytes.clone()], number_of_results)
                    .map_err(|e| miette::miette!("{e}"))?;
                Ok(results
                    .into_read_only()
                    .into_par_iter()
                    .flat_map(|(_x, y)| {
                        y.into_read_only()
                            .into_par_iter()
                            .filter_map(|(_a, b)| serde_json::from_slice(&b).ok())
                            .collect::<Vec<_>>()
                    })
                    .collect())
            }
            EmbeddingModality::Image => {
                let db = self.image_database()?;
                let results = db
                    .query_documents(&[bytes.clone()], number_of_results)
                    .map_err(|e| miette::miette!("{e}"))?;
                Ok(results
                    .into_read_only()
                    .into_par_iter()
                    .flat_map(|(_x, y)| {
                        y.into_read_only()
                            .into_par_iter()
                            .filter_map(|(_a, b)| serde_json::from_slice(&b).ok())
                            .collect::<Vec<_>>()
                    })
                    .collect())
            }
            EmbeddingModality::Text => {
                let db = self.text_database()?;
                let results = db
                    .query_documents(&[bytes.clone()], number_of_results)
                    .map_err(|e| miette::miette!("{e}"))?;
                Ok(results
                    .into_read_only()
                    .into_par_iter()
                    .flat_map(|(_x, y)| {
                        y.into_read_only()
                            .into_par_iter()
                            .filter_map(|(_a, b)| serde_json::from_slice(&b).ok())
                            .collect::<Vec<_>>()
                    })
                    .collect())
            }
        }
    }

    /// Fetch an embedding file associated with a post.
    ///
    /// # Arguments
    ///
    /// * `ticket` - A ticket for the replica containing the file to retrieve.
    ///
    /// * `path` - The path to the file to retrieve.
    ///
    /// * `uri` - The URI associated with the OkuNet post.
    pub async fn fetch_post_embeddings(
        &self,
        ticket: &DocTicket,
        author_id: &AuthorId,
        uri: &str,
    ) -> miette::Result<()> {
        let path: &PathBuf = &OkuNote::embedding_path_from_url(&uri.to_string()).into();
        let archive_path: &PathBuf = &OkuNote::archive_path_from_url(&uri.to_string()).into();
        if let Ok(embedding_bytes) = self
            .fetch_file_with_ticket(ticket, path, &Some(home_replica_filters()))
            .await
        {
            if let Ok(bytes) = self
                .fetch_file_with_ticket(ticket, archive_path, &Some(home_replica_filters()))
                .await
            {
                match self.bytes_to_embedding_modality(&bytes)? {
                    EmbeddingModality::Audio => {
                        let embedding =
                            serde_json::from_str::<Embedding<DIM_VIT_BASE_PATCH16_224>>(
                                String::from_utf8_lossy(&embedding_bytes).as_ref(),
                            )
                            .into_diagnostic()?;
                        let db = self.audio_database()?;
                        db.insert_records(
                            &vec![embedding],
                            &vec![serde_json::to_string(&(author_id, uri))
                                .into_diagnostic()?
                                .into()],
                        )
                        .map_err(|e| miette::miette!("{e}"))?;
                        db.deduplicate().map_err(|e| miette::miette!("{e}"))?;
                    }
                    EmbeddingModality::Image => {
                        let embedding =
                            serde_json::from_str::<Embedding<DIM_VIT_BASE_PATCH16_224>>(
                                String::from_utf8_lossy(&embedding_bytes).as_ref(),
                            )
                            .into_diagnostic()?;
                        let db = self.image_database()?;
                        db.insert_records(
                            &vec![embedding],
                            &vec![serde_json::to_string(&(author_id, uri))
                                .into_diagnostic()?
                                .into()],
                        )
                        .map_err(|e| miette::miette!("{e}"))?;
                        db.deduplicate().map_err(|e| miette::miette!("{e}"))?;
                    }
                    EmbeddingModality::Text => {
                        let embedding = serde_json::from_str::<Embedding<DIM_BGESMALL_EN_1_5>>(
                            String::from_utf8_lossy(&embedding_bytes).as_ref(),
                        )
                        .into_diagnostic()?;
                        let db = self.text_database()?;
                        db.insert_records(
                            &vec![embedding],
                            &vec![serde_json::to_string(&(author_id, uri))
                                .into_diagnostic()?
                                .into()],
                        )
                        .map_err(|e| miette::miette!("{e}"))?;
                        db.deduplicate().map_err(|e| miette::miette!("{e}"))?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Fetch an archived copy of a document.
    ///
    /// # Arguments
    ///
    /// * `author_id` - The authorship ID of the OkuNet user who archived the document.
    ///
    /// * `uri` - The URI of the document.
    ///
    /// # Returns
    ///
    /// The archived copy of the document.
    pub async fn fetch_archive(&self, author_id: &AuthorId, uri: &str) -> anyhow::Result<Bytes> {
        let path: &PathBuf = &OkuNote::archive_path_from_url(&uri.to_string()).into();
        let ticket = self.resolve_author_id(author_id).await?;
        self.fetch_file_with_ticket(&ticket, path, &Some(home_replica_filters()))
            .await
    }
}
