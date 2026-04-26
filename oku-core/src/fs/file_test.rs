#[cfg(test)]
mod tests {
    use std::{path::PathBuf, str::FromStr};

    #[tokio::test]
    async fn test_read_file() -> Result<(), Box<dyn std::error::Error>> {
        let file_contents = "Hello, World!";
        println!("A");
        let file_path = PathBuf::from_str("/test.txt")?;
        println!("B");
        let node = crate::fs::OkuFs::start(
            #[cfg(feature = "fuse")]
            None,
            #[cfg(feature = "persistent")]
            false,
        )
        .await?;
        println!("C");
        let replica_id = node.create_replica().await?;
        println!("D");
        node.create_or_modify_file(&replica_id, &file_path, file_contents)
            .await?;
        println!("E");
        assert_eq!(
            node.read_file(&replica_id, &file_path).await?,
            file_contents
        );
        Ok(())
    }
}
