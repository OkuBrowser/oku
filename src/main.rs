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
use directories_next::ProjectDirs;

use futures::TryStreamExt;
use gtk::prelude::BuilderExtManual;
use gtk::ButtonExt;
use gtk::EntryExt;
use gtk::WidgetExt;
use ipfs_api::IpfsClient;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use webkit2gtk::{WebViewExt};

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref PROJECT_DIRECTORIES: ProjectDirs =
        ProjectDirs::from("org", "Emil Sayahi", "Oku").unwrap();
}

//#[actix_rt::main]
fn main() {
    let cache_directory = PROJECT_DIRECTORIES.cache_dir().to_str().unwrap();
    let client = IpfsClient::default();

    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }
    let glade_src = include_str!("window.glade");
    let web_kit = webkit2gtk::WebViewBuilder::new();
    web_kit.build();
    //let web_kit_functions = webkit2gtk::WebViewExt::
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::Window = builder.get_object("window").unwrap();
    let refresh_button: gtk::Button = builder.get_object("refresh_button").unwrap();
    //let webkit_settings: webkit2gtk_sys::WebKitSettings = builder.get_object("webkit_settings").unwrap();
    let nav_entry: gtk::Entry = builder.get_object("nav_entry").unwrap();
    let web_view: webkit2gtk::WebView = builder.get_object("webkit_view").unwrap();
    let _web_settings: webkit2gtk::Settings = builder.get_object("webkit_settings").unwrap();

    // refresh_button.connect_clicked(move |_| {
    //     dialog.run();
    //     dialog.hide();
    // });

    refresh_button.connect_clicked(move |_refresh_button| {
        let hash = nav_entry.get_text().to_string();
        let local_directory = format!("{}/{}", cache_directory, hash);
        // rt.spawn(future::lazy(|_| {
        //     get_from_hash(client.clone(), hash, local_directory.clone());
        //  }));

        get_from_hash(client.clone(), hash, local_directory.clone());
        // let mut rt = Runtime::new().unwrap();
        // rt.block_on(future);
        // let runtime = Builder::new_multi_thread()
        //     .worker_threads(4)
        //     .thread_name("oku-thread")
        //     .thread_stack_size(3 * 1024 * 1024)
        //     .build()
        //     .unwrap();

        // runtime.block_on(future);
        web_view.load_uri(&format!("file:///{}/index.html", &local_directory));
        //web_view.load_uri("https://crates.io/");
        println!("Loading: {} … ", web_view.get_uri().unwrap().to_string());
    });

    window.show_all();

    gtk::main();

    //let test = "bafybeidd5ronzlgzm4t2upk32zxzxofmyoxv4sdwsded5kc5i4enf7myoe".to_string();
    //get_from_hash(&client, test, cache_directory).await;
}

fn get_from_hash(client: IpfsClient, hash: String, local_directory: String) {
    let mut hierarchy = HashMap::new();
    hierarchy.insert(hash.to_owned(), local_directory.to_owned());
    //let mut rt = actix_rt::Runtime::new().unwrap();
    let mut sys = actix_rt::System::new("name: T");
    //let sys_man = actix_rt::System::current();
    sys.block_on(async move {
        ipfs_download_directory(
            &client,
            local_directory.to_owned(),
            hash.to_owned(),
            hierarchy,
        )
        .await;
        println!("{}", local_directory.clone());
    });
    //sys.run().unwrap();
    //sys_man.stop();
    // ipfs_download_directory(
    //     &client,
    //     local_directory.to_owned(),
    //     hash.to_owned(),
    //     hierarchy,
    // )
    // .await;
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
            match link_type.as_str() {
                "Directory" => {
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
                "File" => {
                    ipfs_download_file(
                        &client,
                        link.hash,
                        format!("{}/{}", hierarchy.get(&directory_hash).unwrap(), link.name),
                    )
                    .await;
                }
                _ => {}
            }
        }
    }
}
