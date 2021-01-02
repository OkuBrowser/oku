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

use directories_next::ProjectDirs;
use gtk::Inhibit;
use percent_encoding::percent_decode_str;

use glib::clone;
use gtk::prelude::BuilderExtManual;
use gtk::ButtonExt;
use gtk::EntryExt;
use gtk::WidgetExt;
use url::{Position, Url};
use webkit2gtk::WebViewExt;

#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref PROJECT_DIRECTORIES: ProjectDirs =
        ProjectDirs::from("org", "Emil Sayahi", "Oku").unwrap();
}

fn connect(nav_entry: gtk::Entry, web_view: webkit2gtk::WebView)
{
    let mut nav_text = nav_entry.get_text().to_string();

    if !nav_text.contains("://")
    {
        nav_text = format!("http://{}", nav_text);
    }

    let url = Url::parse(&nav_text).unwrap();

    match url.scheme()
    {
        "ipfs" => {
            let hash = &url[Position::BeforeHost..];
            let decoded_hash = percent_decode_str(&hash.to_owned()).decode_utf8().unwrap().to_string();
            let split_hash: Vec<&str> = decoded_hash.split('/').collect();
            let path = &decoded_hash.replacen(split_hash[0], "", 1).replacen('/', "", 1);
            let gateway_url = format!("http://{}.ipfs.localhost:8080/{}", split_hash[0], path);
            web_view.load_uri(&gateway_url);
            println!("Loading: {} … ", &gateway_url);
        }
        _ => {
            web_view.load_uri(&nav_text);
            println!("Loading: {} … ", &nav_text);
        }
    }
}

fn on_load(nav_entry: gtk::Entry, web_view: webkit2gtk::WebView)
{
    let mut nav_text = nav_entry.get_text().to_string();

    if !nav_text.contains("://")
    {
        nav_text = format!("http://{}", nav_text);
    }
    println!("{}", nav_text);

    let parsed_url = Url::parse(&nav_text).unwrap();
    let mut url = web_view.get_uri().unwrap().to_string();
    let cid = parsed_url[Position::BeforeHost..].split('/').collect::<Vec<&str>>()[0];
    if url.starts_with(&format!("http://{}.ipfs.localhost:8080/", cid))
    {
        url = url.replacen("http://", "ipfs://", 1).replacen(".ipfs.localhost:8080", "", 1);
    }
    nav_entry.set_text(&url);
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let glade_src = include_str!("window.glade");
    let web_kit = webkit2gtk::WebViewBuilder::new();
    web_kit.build();
    let builder = gtk::Builder::from_string(glade_src);

    let window: gtk::Window = builder.get_object("window").unwrap();
    let go_button: gtk::Button = builder.get_object("go_button").unwrap();
    let nav_entry: gtk::Entry = builder.get_object("nav_entry").unwrap();
    let web_view: webkit2gtk::WebView = builder.get_object("webkit_view").unwrap();

    nav_entry.connect_activate(clone!(@weak web_view, @weak nav_entry => move |_| {
        connect(nav_entry, web_view)
    }));
    go_button.connect_clicked(clone!(@weak web_view, @weak nav_entry => move |_| {
        connect(nav_entry, web_view);
    }));
    web_view.connect_load_changed(clone!(@weak web_view, @weak nav_entry => move |_, _| {
        on_load(nav_entry, web_view)
    }));

    window.show_all();
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    gtk::main();
}
