#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    #[tokio::test]
    async fn test_one_file_operations() -> Result<(), Box<dyn std::error::Error>> {
        let file_contents = "Hello, World!";
        let file_path = PathBuf::from_str("/test.txt")?;
        let node = crate::fs::OkuFs::start(
            #[cfg(feature = "fuse")]
            None,
            #[cfg(feature = "persistent")]
            false,
        )
        .await?;
        let replica_id = node.create_replica().await?;

        // Test creation and reading
        let file_hash = node
            .create_or_modify_file(&replica_id, &file_path, file_contents)
            .await?;
        assert_eq!(
            node.read_file(&replica_id, &file_path).await?,
            file_contents
        );

        // Test listing
        let file_list = node.list_files(&replica_id, &None).await?;
        assert_eq!(1, file_list.len());
        assert_eq!(Some(file_hash), file_list.first().map(|x| x.content_hash()));

        // Test entry retrieval
        let file_entry = node.get_entry(&replica_id, &file_path).await?;
        assert_eq!(Some(&file_entry), file_list.first());

        // Test retrieving all entries from a file
        let file_entries = node.get_entries(&replica_id, &file_path).await?;
        assert_eq!(Some(&file_entry), file_entries.first());

        // Test retrieving oldest timestamp
        let oldest_timestamp = node
            .get_oldest_entry_timestamp(&replica_id, &file_path)
            .await?;
        assert_eq!(file_entry.timestamp(), oldest_timestamp);

        Ok(())
    }

    // #[tokio::test]
    // async fn test_one_file_move() -> Result<(), Box<dyn std::error::Error>> {
    //     todo!();
    // }

    // #[tokio::test]
    // async fn test_multiple_file_operations() -> Result<(), Box<dyn std::error::Error>> {
    //     todo!();
    // }
}
