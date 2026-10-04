// Detect the rustc channel
//
use rustc_version::{ version_meta, Channel };

fn main()
{
	// Needed to avoid warnings for:
	// https://doc.rust-lang.org/nightly/rustc/check-cfg/cargo-specifics.html
	//
	println!("cargo::rustc-check-cfg=cfg(stable)");
	println!("cargo::rustc-check-cfg=cfg(beta)");
	println!("cargo::rustc-check-cfg=cfg(nightly)");
	println!("cargo::rustc-check-cfg=cfg(rustc_dev)");

	// Set cfg flags depending on release channel
	//
	match version_meta().unwrap().channel
	{
		Channel::Stable  => println!( "cargo:rustc-cfg=stable"    ),
		Channel::Beta    => println!( "cargo:rustc-cfg=beta"      ),
		Channel::Nightly => println!( "cargo:rustc-cfg=nightly"   ),
		Channel::Dev     => println!( "cargo:rustc-cfg=rustc_dev" ),
	}

	println!("cargo:rerun-if-changed=build.rs");
}
