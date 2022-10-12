use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use anyhow::Result;
use binrw::{BinReaderExt, VecArgs};

use crate::{
    oodle,
    structs::{BlockHeader, EntryHeader, PackageHeader},
};

pub const BLOCK_SIZE: usize = 0x40000;

pub struct Package {
    pub header: PackageHeader,
    pub entries: Vec<EntryHeader>,
    pub blocks: Vec<BlockHeader>,

    filename_base: String,
}

impl Package {
    pub fn read<R>(filename: String, r: &mut R) -> Result<Self>
    where
        R: Read + Seek,
    {
        r.seek(SeekFrom::Start(0))?;
        let header: PackageHeader = r.read_be()?;

        r.seek(SeekFrom::Start(header.entry_table_offset as u64))?;
        let entries = r.read_be_args(
            VecArgs::builder()
                .count(header.entry_table_size as usize)
                .finalize(),
        )?;

        r.seek(SeekFrom::Start(header.block_table_offset as u64))?;
        let blocks = r.read_be_args(
            VecArgs::builder()
                .count(header.block_table_size as usize)
                .finalize(),
        )?;

        let last_underscore_pos = filename.rfind('_').unwrap();
        let filename_base = filename[..last_underscore_pos].to_owned();

        Ok(Package {
            header,
            entries,
            blocks,
            filename_base,
        })
    }

    pub fn get_block_raw(&self, block_index: usize) -> Result<Vec<u8>> {
        let bh = &self.blocks[block_index];
        // println!(
        //     "Opening {}",
        //     format!("{}_{}.pkg", self.filename_base, bh.patch_id)
        // );
        let mut f = File::open(format!("{}_{}.pkg", self.filename_base, bh.patch_id))?;

        f.seek(SeekFrom::Start(bh.offset as u64))?;
        let mut data = vec![0u8; bh.size as usize];
        f.read_exact(&mut data)?;

        Ok(data)
    }

    pub fn get_block(&self, block_index: usize) -> Result<Vec<u8>> {
        let bh = &self.blocks[block_index];
        let blockdata = self.get_block_raw(block_index)?;

        if (bh.flags & 0x100) != 0 {
            let mut decomp_buffer = vec![0; BLOCK_SIZE];
            let decompressed_size = oodle::decompress(&blockdata, &mut decomp_buffer);
            if decompressed_size == 0 {
                println!("Oodle decompression failed");
                anyhow::bail!("Oodle decompression failed");
            }
            Ok(decomp_buffer)
        } else {
            Ok(blockdata)
        }
    }
}
