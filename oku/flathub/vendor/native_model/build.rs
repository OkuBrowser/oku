extern crate skeptic;

use skeptic::{generate_doc_tests, markdown_files_of_directory};

fn main() {
    {
        let mut mdbook_files = markdown_files_of_directory("doc/");
        mdbook_files.push("README.md".into());
        generate_doc_tests(&mdbook_files);
    }
}
