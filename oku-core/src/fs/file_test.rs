#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    #[tokio::test]
    async fn test_one_file_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
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
            .create_file(&replica_id, &file_path, file_contents)
            .await?;
        assert_eq!(
            node.read_file(&replica_id, &file_path, &None, &None)
                .await?,
            file_contents
        );

        // Test listing
        let file_list = node.list_files(&replica_id, &None).await?;
        assert_eq!(1, file_list.len());
        assert_eq!(Some(file_path.clone()), file_list.first().cloned());

        // Test entry retrieval
        let file_entry = node.get_entry(&replica_id, &file_path).await?;
        assert_eq!(file_hash, file_entry.content_hash());

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

    #[tokio::test]
    async fn test_one_file_move() -> Result<(), Box<dyn std::error::Error>> {
        let file_contents = "Hello, World!";
        let first_file_path = PathBuf::from_str("/test.txt")?;
        let second_file_path = PathBuf::from_str("/dir/test.txt")?;

        let node = crate::fs::OkuFs::start(
            #[cfg(feature = "fuse")]
            None,
            #[cfg(feature = "persistent")]
            false,
        )
        .await?;
        let replica_a = node.create_replica().await?;
        let replica_b = node.create_replica().await?;

        // The replicas begin with no files
        let replica_a_list_one = node.list_files(&replica_a, &None).await?;
        let replica_b_list_one = node.list_files(&replica_b, &None).await?;
        assert_eq!(0, replica_a_list_one.len());
        assert_eq!(0, replica_b_list_one.len());

        // Add a file to replica A
        let file_hash = node
            .create_file(&replica_a, &first_file_path, file_contents)
            .await?;
        assert_eq!(
            node.read_file(&replica_a, &first_file_path, &None, &None)
                .await?,
            file_contents
        );
        let replica_a_list_two = node.list_files(&replica_a, &None).await?;
        let replica_b_list_two = node.list_files(&replica_b, &None).await?;
        assert_eq!(1, replica_a_list_two.len());
        assert_eq!(0, replica_b_list_two.len());

        // Move it to replica B
        let (first_moved_file_hash, _) = node
            .move_file(&replica_a, &first_file_path, &replica_b, &first_file_path)
            .await?;
        assert_eq!(file_hash, first_moved_file_hash);
        assert_eq!(
            node.read_file(&replica_b, &first_file_path, &None, &None)
                .await?,
            file_contents
        );
        let replica_a_list_three = node.list_files(&replica_a, &None).await?;
        let replica_b_list_three = node.list_files(&replica_b, &None).await?;
        assert_eq!(0, replica_a_list_three.len());
        assert_eq!(1, replica_b_list_three.len());

        // Move it within replica B
        let (second_moved_file_hash, _) = node
            .move_file(&replica_b, &first_file_path, &replica_b, &second_file_path)
            .await?;
        assert_eq!(file_hash, second_moved_file_hash);
        assert_eq!(
            node.read_file(&replica_b, &second_file_path, &None, &None)
                .await?,
            file_contents
        );
        let replica_a_list_four = node.list_files(&replica_a, &None).await?;
        let replica_b_list_four = node.list_files(&replica_b, &None).await?;
        assert_eq!(0, replica_a_list_four.len());
        assert_eq!(1, replica_b_list_four.len());
        Ok(())
    }

    // #[tokio::test]
    // async fn test_multiple_file_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    //     todo!();
    // }

    // #[tokio::test]
    // async fn test_partial_read() -> Result<(), Box<dyn std::error::Error>> {
    //     todo!();
    // }
}
