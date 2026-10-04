use std::path::PathBuf;

use clap::Parser;
use libp2p_identity::PeerId;
use rust_ipns::Record;

#[derive(Debug, clap::Parser)]
#[clap(name = "inspect")]
struct Opt {
    file: PathBuf,
    #[clap(long)]
    key: Option<PeerId>,
}

fn main() -> std::io::Result<()> {
    let opt = Opt::parse();

    let bytes = std::fs::read(opt.file)?;

    let record = Record::decode(bytes)?;

    let value = String::from_utf8_lossy(record.value());

    let validity_type = record.validity_type();

    let validity = record.validity()?;

    let seq = record.sequence();

    let ttl = record.ttl();

    let sig_v1 = record.has_signature_v1();
    let sig_v2 = record.has_signature_v2();

    println!("Value: {value}");
    println!("Validity Type: {validity_type}");
    println!("Validity: {validity}");
    println!("Sequence: {seq}");
    println!("TTL: {ttl}");

    print!("Signature Type:");

    match (sig_v1, sig_v2) {
        (true, true) => println!("V1+V2"),
        (true, false) => println!("V1"),
        (false, true) => println!("V2"),
        (false, false) => println!("N/A"),
    };

    if let Some(peer_id) = opt.key {
        match record.verify(peer_id).is_ok() {
            true => println!("Signature Verified"),
            false => println!("Signature is Invalid"),
        };
    } else {
        println!("Record has not been validated");
    }

    Ok(())
}
