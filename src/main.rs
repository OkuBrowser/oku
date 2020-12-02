/*
    This file is part of Oku.

    Oku is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Oku is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with Oku.  If not, see <https://www.gnu.org/licenses/>.
*/
use async_recursion::async_recursion;
use directories::ProjectDirs;
use futures::TryStreamExt;
use ipfs_api::IpfsClient;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[actix_rt::main]
async fn main() {
    let project_directories = ProjectDirs::from("org", "Emil Sayahi", "Oku").unwrap();
    let client = IpfsClient::default();
    let test = "bafybeicqqzygir3ysia2ivmhdwi6znvuivrl2yqtmapp57fuz4zk6ugweq".to_string();
    let local_directory = format!(
        "{}/{}",
        project_directories.cache_dir().to_str().unwrap(),
        test
    );
    let mut hierarchy = HashMap::new();
    hierarchy.insert(test.to_owned(), local_directory.to_owned());
    ipfs_download_directory(
        &client,
        local_directory.to_owned(),
        test.to_owned(),
        hierarchy,
    )
    .await;
}

async fn ipfs_download_file(client: &IpfsClient, file_hash: String, file_path: String) {
    match client
        .cat(&file_hash)
        .map_ok(|chunk| chunk.to_vec())
        .try_concat()
        .await
    {
        Ok(res) => {
            println!("Writing: {} ({}) … ", file_path, file_hash);
            fs::create_dir_all(Path::new(&file_path[..]).parent().unwrap()).unwrap();
            fs::write(file_path, &res).unwrap();
        }
        Err(e) => eprintln!(
            "Failed to obtain file: {} ({})\nError: {:#?}",
            file_path, file_hash, e
        ),
    }
}

#[async_recursion(?Send)]
async fn ipfs_download_directory(
    client: &IpfsClient,
    directory: String,
    directory_hash: String,
    mut hierarchy: HashMap<String, String>,
) {
    hierarchy.insert(directory_hash.clone(), directory.clone());
    let directory_object = client.file_ls(&directory_hash).await.unwrap().objects;
    for object in directory_object {
        for link in object.1.links {
            let link_type = &link.typ.clone().unwrap();
            match &link_type.as_str() {
                &"Directory" => {
                    let sub_directory =
                        format!("{}/{}", hierarchy.get(&directory_hash).unwrap(), link.name);
                    ipfs_download_directory(
                        &client,
                        sub_directory.to_owned(),
                        link.hash.clone(),
                        hierarchy.clone(),
                    )
                    .await;
                    hierarchy.insert(link.hash, sub_directory);
                }
                &"File" => {
                    ipfs_download_file(
                        &client,
                        link.hash,
                        format!("{}/{}", hierarchy.get(&directory_hash).unwrap(), link.name),
                    )
                    .await;
                }
                &_ => {}
            }
        }
    }
}
