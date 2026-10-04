use std::env;
use std::fs;
use std::path::Path;

use askama::Template;

mod templates;
use crate::templates::*;

pub enum Modes {
    Serial,
    Parallel,
    Async,
}

impl Modes {
    fn as_str(&self) -> &str {
        match self {
            Modes::Serial => "serial",
            Modes::Parallel => "parallel",
            Modes::Async => "async",
        }
    }
}

fn main() -> std::io::Result<()> {
    let template_dir = "templates";
    println!("cargo:rerun-if-changed={template_dir}");
    for entry in std::fs::read_dir(template_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }

    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR not set");

    let mut modes = vec![];
    if env::var("CARGO_FEATURE_SERIAL").is_ok() {
        modes.push(Modes::Serial);
    }
    if env::var("CARGO_FEATURE_PARALLEL").is_ok() {
        modes.push(Modes::Parallel);
    }
    if env::var("CARGO_FEATURE_ASYNC").is_ok() {
        modes.push(Modes::Async);
    }

    for mode in modes {
        let mode = mode.as_str();
        let mode_dir = Path::new(&out_dir).join(mode);
        fs::create_dir_all(&mode_dir)?;

        let content = FuseDriverTemplate { mode }.render()?;
        fs::write(mode_dir.join("fuse_driver.rs"), content)?;

        let content = FuseHandlerTemplate { mode }.render()?;
        fs::write(mode_dir.join("fuse_handler.rs"), content)?;

        let content = MountingTemplate { mode }.render()?;
        fs::write(mode_dir.join("mounting.rs"), content)?;

        let content = FuseLibTemplate { mode }.render()?;
        fs::write(mode_dir.join("fuse_lib.rs"), content)?;
    }

    Ok(())
}
