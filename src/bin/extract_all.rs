use rsa::{pkcs1::DecodeRsaPublicKey, PaddingScheme, PublicKey};
use sha2::digest::DynDigest;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};

fn main() -> anyhow::Result<()> {
    let filename = std::env::args().nth(1).unwrap();
    let mut f = File::open(filename.clone())?;
    let pkg = destinypkg::package::Package::read(filename.clone(), &mut f)?;

    for (i, eh) in pkg
        .entries
        .iter()
        .enumerate()
    {
        println!(
            "{:04x}_{}_{} - ref 0x{:x?} type {} subtype {} size {}",
            pkg.header.pkg_id,
            pkg.header.patch_id,
            i,
            eh.reference,
            eh.num_type,
            eh.num_subtype,
            eh.file_size
        );
        println!("Extracting {:x?}", eh);
        match pkg.get_entry_data(eh) {
            Ok(d) => {
                let mut f = File::create(format!("out/{}_{}.bin", pkg.header.pkg_id, i))?;
                f.write_all(&d)?;
            }
            Err(e) => {
                println!("Extraction failed: {}", e);
            }
        }
    }

    Ok(())
}
