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
use futures::TryStreamExt;
use ipfs_api::IpfsClient;
use std::io::{self, Write};

#[actix_rt::main]
async fn main() {
    let client = IpfsClient::default();
    ipfs_download(&client, "/ipfs/bafybeicqqzygir3ysia2ivmhdwi6znvuivrl2yqtmapp57fuz4zk6ugweq/Makefile".to_string()).await;
}

async fn ipfs_download(client: &IpfsClient, address: String)
{
    match client.get(&address).map_ok(|chunk| chunk.to_vec()).try_concat().await
    {
    
        Ok(res) => {
            let out = io::stdout();
            let mut out = out.lock();
    
            out.write_all(&res).unwrap();
        }
        Err(e) => eprintln!("error getting file: {}", e)    
    }
}