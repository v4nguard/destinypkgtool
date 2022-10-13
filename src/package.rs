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
                anyhow::bail!("Oodle decompression failed");
            }
            Ok(decomp_buffer)
        } else {
            Ok(blockdata)
        }
    }

    pub fn get_entry_data(&self, entry: &EntryHeader) -> Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(entry.file_size as usize);
        let mut current_offset = 0usize;
        let mut current_block = entry.starting_block;

        while current_offset < entry.file_size as usize {
            let block_data = self.get_block(current_block as usize)?;
            let remaining_bytes = entry.file_size as usize - current_offset;

            if current_block == entry.starting_block {
                // If we're on the starting block, we might not start at the beginning of the block
                let copy_size;
                let block_start_offset = (entry.starting_block_offset * 16) as usize;
                let block_remaining = block_data.len() - block_start_offset;
                if block_remaining < remaining_bytes {
                    copy_size = block_remaining;
                } else {
                    copy_size = remaining_bytes;
                }

                buffer.extend_from_slice(
                    &block_data[block_start_offset..block_start_offset + copy_size],
                );

                current_offset += copy_size;
            } else if remaining_bytes < block_data.len() {
                // If the block has more bytes than we need, it means we're on the last block
                buffer.extend_from_slice(&block_data[..remaining_bytes]);
                current_offset += remaining_bytes;
            } else {
                // If the previous 2 conditions failed, it means this whole block belongs to the file
                buffer.extend_from_slice(&block_data[..]);
                current_offset += block_data.len();
            }

            current_block += 1;
        }

        Ok(buffer)
    }
}
