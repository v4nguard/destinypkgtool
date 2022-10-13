#![feature(seek_stream_len)]

use rsa::{pkcs1::DecodeRsaPublicKey, PaddingScheme, PublicKey};
use sha2::digest::DynDigest;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

macro_rules! check_result {
    ($m:expr, $v:expr) => {
        print!("{:18} - ", $m);
        if $v {
            println!("{}   ", ansi_term::Color::Green.bold().paint("ok"));
        } else {
            println!("{}   ", ansi_term::Color::Red.bold().paint("FAILED"));
        }
    };
}

macro_rules! print_progress {
    ($m:expr, $current:expr, $count:expr) => {
        print!(
            "\r{:18} - {:.0}%",
            $m,
            ($current as f32 / $count as f32) * 100.0
        );
    };
}

fn main() -> anyhow::Result<()> {
    let filename = std::env::args().nth(1).unwrap();
    let mut f = File::open(filename.clone())?;
    let pkg = destinypkg::package::Package::read(filename, &mut f)?;

    println!("Package info:");
    println!(" PKG ID:       {:04x}", pkg.header.pkg_id);
    println!(" Patch ID:     {}", pkg.header.patch_id);
    println!(" Language:     {:?}", pkg.header.language);
    println!(
        " Build date:   {}",
        chrono::NaiveDateTime::from_timestamp(pkg.header.build_time as i64, 0)
    );
    println!(" Tool string:  {}", pkg.header.tool_string);
    println!(" Entries:      {}", pkg.header.entry_table_size);
    println!(
        " Blocks:       {} ({} compressed)",
        pkg.header.block_table_size,
        pkg.blocks.iter().filter(|bh| bh.flags & 0x100 != 0).count()
    );

    println!();

    check_result!("File size", pkg.header.file_size as u64 == f.stream_len()?);

    let mut hasher = sha2::Sha256::default();
    let mut headerdata = [0u8; 320];
    f.seek(SeekFrom::Start(0))?;
    f.read_exact(&mut headerdata)?;

    hasher.update(&headerdata);
    let headerhash = hasher.finalize_reset();

    let pubkey = rsa::RsaPublicKey::from_pkcs1_der(include_bytes!("../../pkg_pubkey.bin"))?;
    f.seek(SeekFrom::Start(pkg.header.header_signature_offset as u64))?;
    let mut sigdata = [0u8; 256];
    f.read_exact(&mut sigdata)?;

    let r = pubkey.verify(
        PaddingScheme::PSS {
            digest: Box::new(sha2::Sha256::default()),
            salt_len: Some(16),
        },
        &headerhash,
        &sigdata,
    );

    check_result!("Header signature", r.is_ok());

    let mut hasher = sha1::Sha1::default();
    f.seek(SeekFrom::Start(pkg.header.entry_table_offset as u64))?;
    let mut entrytable = vec![0u8; pkg.header.entry_table_size as usize * 0x10];
    f.read_exact(&mut entrytable)?;
    hasher.update(&entrytable);

    check_result!(
        "Entry table hash",
        hasher.finalize_reset() == Box::new(pkg.header.entry_table_hash)
    );

    f.seek(SeekFrom::Start(pkg.header.block_table_offset as u64))?;
    let mut blocktable = vec![0u8; pkg.header.block_table_size as usize * 32];
    f.read_exact(&mut blocktable)?;
    hasher.update(&blocktable);
    check_result!(
        "Block table hash",
        hasher.finalize_reset() == Box::new(pkg.header.block_table_hash)
    );

    let mut failed_blocks = 0;
    for i in 0..pkg.blocks.len() {
        print_progress!("Block hashes", i, pkg.blocks.len());
        let blockdata = pkg.get_block_raw(i)?;
        hasher.update(&blockdata);
        if hasher.finalize_reset() != Box::new(pkg.blocks[i].hash) {
            failed_blocks += 1;
        }
    }
    print!("\r");
    check_result!("Block hashes", failed_blocks == 0);
    if failed_blocks != 0 {
        println!(
            "\tFailed blocks: {} out of {}",
            failed_blocks,
            pkg.blocks.len()
        );
    }

    let mut failed_blocks = 0;
    for (i, bh) in pkg.blocks.iter().enumerate() {
        if (bh.flags & 0x100) != 0 {
            print_progress!("Compressed blocks", i, pkg.blocks.len());
            let r = pkg.get_block(i);
            if r.is_err() {
                failed_blocks += 1;
            }
        }
    }
    print!("\r");
    check_result!("Compressed blocks", failed_blocks == 0);
    if failed_blocks != 0 {
        println!(
            "\tFailed blocks: {} out of {}",
            failed_blocks,
            pkg.blocks.iter().filter(|bh| bh.flags & 0x100 != 0).count()
        );
    }

    // Check if all file entries point to valid blocks
    let mut bad_entries = 0;
    for (i, eh) in pkg.entries.iter().enumerate() {
        print_progress!("File entries", i, pkg.entries.len());
        if eh.starting_block as usize > pkg.blocks.len() {
            bad_entries += 1;
            continue;
        }

        if pkg.get_entry_data(eh).is_err() {
            bad_entries += 1;
            continue;
        }
    }
    print!("\r");
    check_result!("File entries", bad_entries == 0);
    if bad_entries != 0 {
        println!(
            "\tFailed entries: {} out of {}",
            bad_entries,
            pkg.entries.len()
        );
    }

    Ok(())
}
