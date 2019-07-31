//! # A Simple BitMap Implementation
//!
//! This module implements a simple persistent bitmap.  The
//! bitmap is maintained in a single file.  The caller is
//! responsible for path management and file creation.
//!
//! The bit map is maintained in memory and on disk as a
//! sequence of blocks.  Each block is self-identifying
//! and checksummed to help handle problems with storage
//! systems.
//!
//! This bitmap is intended for the ledger, so it allows
//! the caller to append set bits, but not zero bits, as a
//! minor check of correctness.
//!

use sodiumoxide::crypto::hash::sha256;
use std::fs::File;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom::End;
use std::io::SeekFrom::Start;
use std::io::Write;
use std::mem;
use std::slice::from_raw_parts;
use std::slice::from_raw_parts_mut;
use super::append_only_merkle::timestamp;

// Returns Err(Error::new...).
macro_rules! se {
    ($($x:tt)+) => { Err(Error::new(ErrorKind::Other, format!($($x)+))) }
}

// Write a log entry to stdout.
macro_rules! log {
  ($($x:tt)+) => { print!("{}    ", timestamp()); println!($($x)+); }
}

// Write a log entry to stdout.
//
// This macro is used only for debugging simple problems
// with the basic mapping logic.
//

macro_rules! verbose_log {
  ($($x:tt)+) => { }
  // ($($x:tt)+) => { print!("{}    ", timestamp()); println!($($x)+); }
}

const CHECK_SIZE: usize = 16;

#[repr(C)]
#[derive(PartialEq)]
struct CheckBlock {
  bytes:  [u8; CHECK_SIZE],
}

// A structure for a checksum on a block.
impl CheckBlock {
  fn new() -> CheckBlock {
    CheckBlock {
      bytes:  [0_u8; CHECK_SIZE],
    }
  }
}

const HEADER_MAGIC:  u32    = 0x0204_0600;
const BIT_INVALID:   u16    =  0;  // This value is used for testing.
const BIT_ARRAY:     u16    =  1;
const BIT_DESC:      u16    =  2;
const HEADER_SIZE:   usize  = 40;

// Define the layout for a block header.
//
// checksum  a checksum over the rest of the block
// magic     a magic number
// count     the count of valid bits in this block
// id        the block index
// contents  the contents type, currently always BIT_ARRAY
//

#[repr(C)]
struct BlockHeader {
  checksum:  CheckBlock, // must be first
  magic:     u32,        // must be second
  count:     u32,
  id:        u64,
  contents:  u16,
  pad_1:     u16,
  pad_2:     u32,
}

impl BlockHeader {
  fn new(block_contents: u16, block_id: u64) -> Result<BlockHeader> {
    if block_contents != BIT_ARRAY && block_contents != BIT_DESC {
      return se!("That content type ({}) is invalid.", block_contents);
    }

    let result =
      BlockHeader {
        checksum:  CheckBlock::new(),
        magic:     HEADER_MAGIC,
        count:     0,
        id:        block_id,
        contents:  block_contents,
        pad_1:     0,
        pad_2:     0,
      };

    Ok(result)
  }

  fn validate(&self, contents: u16, id: u64) -> Result<()> {
    if self.magic != HEADER_MAGIC {
      return se!("Block {} has a bad header mark:  {:x}", id, self.magic);
    }

    if self.count > BLOCK_BITS as u32 {
      return se!("Block {} has a bad count:  {} vs {}", id, self.count, BLOCK_BITS);
    }

    if self.id != id {
      return se!("Block {} has a bad id:  the disk said {}.", id, self.id);
    }

    if self.contents != contents {
      return se!("Block {} has a bad contents type:  {}", id, self.contents);
    }

    if self.pad_1 != 0 {
      return se!("Block {} has an invalid pad_1:  {}", id, self.pad_1);
    }

    if self.pad_2 != 0 {
      return se!("Block {} has an invalid pad_2:  {}", id, self.pad_2);
    }

    Ok(())
  }
}

const BLOCK_SIZE:  usize  = 32 * 1024;
const BITS_SIZE:   usize  = BLOCK_SIZE - HEADER_SIZE;
const BLOCK_BITS:  usize  = BITS_SIZE * 8;

// Define the layout of a block of a bitmap.  The on-disk
// and in-memory layouts are the same.
#[repr(C)]
struct BitBlock {
  header:  BlockHeader,
  bits:    [u8; BITS_SIZE],
}

impl BitBlock {
  // Create a new block header structure.
  fn new(block_contents: u16, block_id: u64) -> Result<BitBlock> {
    let result =
      BitBlock {
        header:  BlockHeader::new(block_contents, block_id)?,
        bits:    [0_u8; BITS_SIZE],
      };

    Ok(result)
  }

  // Compute a checksum for the block.
  fn compute_checksum(&self) -> [u8; CHECK_SIZE] {
    let digest = sha256::hash(self.as_checksummed_region());
    let mut result: [u8; CHECK_SIZE] = Default::default();

    result.clone_from_slice(&digest[0..CHECK_SIZE]);
    result
  }

  // Set the block check bits with the current checksum for the block.
  fn set_checksum(&mut self) {
    self.header.checksum.bytes = self.compute_checksum();
  }    

  // Create a slice for writing a block to disk.
  fn as_ref(&self) -> &[u8] {
    unsafe {
      from_raw_parts((self as *const BitBlock) as *const u8,
                mem::size_of::<BitBlock>())
    }
  }

  // Create a mutable slice for reading a block from disk.
  fn as_mut(&mut self) -> &mut [u8] {
    unsafe {
      from_raw_parts_mut((self as *mut BitBlock) as *mut u8,
                mem::size_of::<BitBlock>())
    }
  }

  // Create a slice corresponding to the part of the block
  // that is checksummed.
  fn as_checksummed_region(&self) -> &[u8] {
    unsafe {
        from_raw_parts(
            (&self.header.magic as *const u32) as *const u8,
            mem::size_of::<BitBlock>() - mem::size_of::<CheckBlock>())
    }
  }

  // Validate the contents of a block from the disk.
  fn validate(&self, contents: u16, id: u64) -> Result<()> {
    self.header.validate(contents, id)?;

    let checksum = self.compute_checksum();

    if self.header.checksum.bytes != checksum {
      return se!("Block {} has a bad checksum.", id);
    }

    Ok(())
  }
}

/// Define the structure for controlling a persistent bitmap.
pub struct BitMap {
  file:    File,
  size:    usize,
  blocks:  Vec<BitBlock>,
  dirty:   Vec<bool>,
}

impl Drop for BitMap {
  fn drop(&mut self) {
    let _ = self.write();
  }
}

impl BitMap {
  /// Create a new bit map.  The caller should pass a File
  /// structure opened to an empty file.
  ///
  /// # Example
  ///````
  /// use std::fs::OpenOptions;
  /// use crate::core::store::bitmap::BitMap;
  ///
  /// let path = "sample_name";
  ///
  /// # let _ = std::fs::remove_file(&path);
  /// let file =
  ///   OpenOptions::new()
  ///     .read(true)
  ///     .write(true)
  ///     .create_new(true)
  ///     .open(&path)
  ///     .unwrap();
  ///              
  /// let mut bitmap =
  ///   match BitMap::create(file) {
  ///     Ok(bitmap) => { bitmap }
  ///     Err(e) => { panic!("create failed:  {}", e); }
  ///   };
  ///
  /// bitmap.set(0);
  ///
  /// if let Err(e) = bitmap.write() {
  ///   panic!("Write failed:  {}", e);
  /// }
  /// # let _ = std::fs::remove_file(&path);
  ///````
  pub fn create(mut data: File) -> Result<BitMap> {
    let file_size = data.seek(End(0))?;

    if file_size != 0 {
      return se!("The file contains data!");
    }

    let result =
      BitMap {
        file:    data,
        size:    0,
        blocks:  Vec::new(),
        dirty:   Vec::new(),
      };

    Ok(result)
  }

  /// Open an existing bitmap.
  ///
  /// # Example
  ///````
  /// use std::fs::OpenOptions;
  /// use crate::core::store::bitmap::BitMap;
  ///
  /// let path = "sample_name";
  ///
  /// # let _ = std::fs::remove_file(&path);
  /// # let file =
  /// #   OpenOptions::new()
  /// #     .read(true)
  /// #     .write(true)
  /// #     .create_new(true)
  /// #     .open(&path)
  /// #     .unwrap();
  /// # drop(file);
  /// let file =
  ///   OpenOptions::new()
  ///     .read(true)
  ///     .write(true)
  ///     .open(&path)
  ///     .unwrap();
  ///
  /// let mut bitmap =
  ///   match BitMap::open(file) {
  ///     Ok(bitmap) => { bitmap }
  ///     Err(e) => { panic!("open failed:  {}", e); }
  ///   };
  ///
  /// bitmap.set(0);
  /// bitmap.set(1);
  /// bitmap.set(2);
  /// bitmap.clear(1);
  ///
  /// if let Err(e) = bitmap.write() {
  ///   panic!("Write failed:  {}", e);
  /// }
  ///````
  pub fn open(mut data: File) -> Result<BitMap> {
    let (count, block_vector, state_vector) = BitMap::read_file(&mut data)?;

    let result =
      BitMap {
        file:    data,
        size:    count,
        blocks:  block_vector,
        dirty:   state_vector,
      };

    Ok(result)
  }

  // Read the contents of a file into memory, checking the
  // validity as we go.
  fn read_file(file: &mut File) -> Result<(usize, Vec<BitBlock>, Vec<bool>)> {
    let mut blocks = Vec::new();
    let mut dirty  = Vec::new();
    let mut count  = 0;

    // Compute the number of blocks in the file.
    let file_size = file.seek(End(0))?;

    if file_size % BLOCK_SIZE as u64 != 0 {
      return se!("That file size ({}) is invalid.", file_size);
    }

    file.seek(Start(0))?;
    let total_blocks = file_size / BLOCK_SIZE as u64;

    // Reserve space in our vectors.
    blocks.reserve(total_blocks as usize);
    dirty .reserve(total_blocks as usize);

    // Read each block.
    for index in 0..total_blocks {
      let mut block = BitBlock::new(BIT_ARRAY, 0).unwrap();

      block.header.contents = BIT_INVALID;

      match file.read_exact(block.as_mut()) {
        Ok(_) => {
          block.validate(BIT_ARRAY, index)?;

          if index != total_blocks - 1 && block.header.count != BLOCK_BITS as u32 {
            return se!("Block {} is not full:  count {}", index, block.header.count);
          }

          count += block.header.count as usize;
          blocks.push(block);
          dirty.push(false);
        }
        Err(e) => {
          return Err(e);
        }
      }
    }

    Ok((count, blocks, dirty))
  }

  /// Query the value of a bit in the map.
  pub fn query(&self, bit: usize) -> Result<bool> {
    if bit >= self.size {
      return se!("That index is out of range ({} vs {}).", bit, self.size);
    }

    let block   = bit / BLOCK_BITS;
    let bit_id  = bit % BLOCK_BITS;
    let index   = bit_id / 8;
    let mask    = 1 << (bit_id % 8);

    verbose_log!("query({}) -> block {}, index {}, mask {}",
      bit, block, index, mask);
    let value = self.blocks[block].bits[index] & mask;
    Ok(value != 0)
  }

  /// Set the given bit.
  pub fn set(&mut self, bit: usize) -> Result<()> {
    if bit > self.size {
      return se!("That index is too large to set ({} vs {}).", bit, self.size);
    }

    self.mutate(bit, 1, true)
  }

  /// Clear the given bit.
  pub fn clear(&mut self, bit: usize) -> Result<()> {
    if bit >= self.size {
      return se!("That index is too large to clear ({} vs {}).", bit, self.size);
    }

    self.mutate(bit, 0, false)
  }

  // Change the value of the given bit, as requested.
  fn mutate(&mut self, bit: usize, value: u8, extend: bool) -> Result<()> {
    if !extend && bit >= self.size {
      return se!("That index ({}) is out of range.", bit);
    }

    let block   = bit / BLOCK_BITS;
    let bit_id  = bit % BLOCK_BITS;
    let index   = bit_id / 8;
    let mask    = 1 << (bit_id % 8);

    if block >= self.blocks.len() {
      self.blocks.push(BitBlock::new(BIT_ARRAY, block as u64)?);
      self.dirty.push(true);
    } else {
      self.dirty[block] = true;
    }

    verbose_log!("mutate({}, {}) -> block {}, index {}, mask {}, BLOCK_BITS {}",
      bit, value, block, index, mask, BLOCK_BITS);

    if value == 0 {
      self.blocks[block].bits[index] &= !mask;
    } else {
      self.blocks[block].bits[index] |=  mask;
    }

    if bit >= self.size {
      self.size = bit + 1;

      self.blocks[block].header.count += 1;

      if self.blocks[block].header.count == BLOCK_BITS as u32 {
        if let Err(e) = self.write_block(block) {
          log!("Error writing block {}:  {}", block, e);
        }
      }
    }

    Ok(())
  }

  /// Return the number of bits in the map.
  pub fn size(&self) -> usize {
    self.size
  }

  /// Write the bitmap to disk.
  pub fn write(&mut self) -> Result<()> {
    for i in 0..self.blocks.len() {
      if self.dirty[i] {
        self.write_block(i)?;
      }
    }

    self.file.sync_all()?;
    Ok(())
  }

  // Write the given block to disk and clear the dirty flag.
  fn write_block(&mut self, index: usize) -> Result<()> {
    let offset = index as u64 * BLOCK_SIZE as u64;
    self.file.seek(Start(offset))?;
    self.blocks[index].set_checksum();
    self.file.write_all(self.blocks[index].as_ref())?;
    self.dirty[index] = false;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use std::fs;
  use std::fs::OpenOptions;
  use std::mem;
  use super::*;

  #[test]
  fn test_header() {
    assert!(mem::size_of::<BlockHeader>() == HEADER_SIZE);

    let id = 24_000;
    let mut header = BlockHeader::new(BIT_ARRAY, id).unwrap();
    assert!(header.contents == BIT_ARRAY);
    assert!(header.id == id);

    if let Err(e) = header.validate(BIT_ARRAY, id) {
      panic!("Validation failed:  {}", e);
    }

    header.magic ^= 1;

    if let Ok(_) = header.validate(BIT_ARRAY, id) {
      panic!("Validation failed to detect a bad mark.");
    }

    header.magic ^= 1;

    header.count = (BLOCK_BITS + 1) as u32;

    if let Ok(_) = header.validate(BIT_ARRAY, id) {
      panic!("Validation failed to detect a bad count.");
    }

    header.count = 0;
    header.id ^= 1;

    if let Ok(_) = header.validate(BIT_ARRAY, id) {
      panic!("Validation failed to detect a bad id.");
    }

    header.id ^= 1;
    header.contents = BIT_INVALID;

    if let Ok(_) = header.validate(BIT_ARRAY, id) {
      panic!("Validation failed to detect a bad contents type.");
    }

    header.contents = BIT_ARRAY;
    header.pad_1 = 1;

    if let Ok(_) = header.validate(BIT_ARRAY, id) {
      panic!("Validation failed to detect a bad pad_1.");
    }

    header.pad_1 = 0;
    header.pad_2 = 1;

    if let Ok(_) = header.validate(BIT_ARRAY, id) {
      panic!("Validation failed to detect a bad pad_2.");
    }

    let header = BlockHeader::new(BIT_DESC, 0).unwrap();
    assert!(header.contents == BIT_DESC);
    assert!(header.id == 0);

    assert!(header.count == 0);
    assert!(header.checksum == CheckBlock::new());

    if let Ok(_) = BlockHeader::new(BIT_INVALID, 0) {
      panic!("An invalid block type was accepted.");
    }
  }

  #[test]
  fn test_block() {
    println!("The block size is {}.", mem::size_of::<BitBlock>());
    println!("The header size is {}.", mem::size_of::<BlockHeader>());
    assert!(mem::size_of::<BlockHeader>() == HEADER_SIZE);
    assert!(mem::size_of::<BitBlock>() == BLOCK_SIZE);
    assert!(BLOCK_SIZE == BITS_SIZE + HEADER_SIZE);
    assert!(BLOCK_SIZE == BLOCK_BITS / 8 + HEADER_SIZE);

    let mut block = BitBlock::new(BIT_DESC, 32).unwrap();
    assert!(block.header.contents == BIT_DESC);
    assert!(block.header.id == 32);

    block.set_checksum();

    if let Err(_) = block.validate(BIT_DESC, 32) {
      panic!("Block validation failed.");
    }
  }

  #[test]
  fn test_basic_bitmap() {
    let path = "basic_bitmap";
    let _    = fs::remove_file(&path);

    let file =
      OpenOptions::new()
        .read      (true)
        .write     (true)
        .create_new(true)
        .open      (&path)
        .unwrap    ();

    let mut bitmap = BitMap::create(file).unwrap();

    if let Err(e) = bitmap.write() {
      panic!("Write failed:  {}", e);
    }

    let file =
      OpenOptions::new()
        .read      (true)
        .write     (true)
        .open      (&path)
        .unwrap    ();

    let mut bitmap = BitMap::open(file).unwrap();

    for i in 0..2 * BLOCK_BITS + 2 {
      bitmap.set(i).unwrap();
      assert!(bitmap.query(i).unwrap() == true);
      assert!(bitmap.size() == i + 1);

      if let Ok(_) = bitmap.query(i + 1) {
        panic!("Index {} should be out of range.", i + 1);
      }
    }

    for i in 0..bitmap.size() {
      if i & 1 == 0 {
        bitmap.clear(i).unwrap();
        assert!(bitmap.query(i).unwrap() == false);
      }
    }

    for i in 0..bitmap.size() {
      assert!(bitmap.query(i).unwrap() == !(i & 1 == 0));
    }

    if let Err(_) = bitmap.write() {
      panic!("Write failed.");
    }

    let bits_initialized = bitmap.size();

    if let Err(e) = bitmap.write() {
      panic!("Write failed:  {}", e);
    }

    let file =
      OpenOptions::new()
        .read      (true)
        .write     (true)
        .open      (&path)
        .unwrap    ();

    let bitmap = BitMap::open(file).unwrap();

    for i in 0..bits_initialized {
      assert!(bitmap.query(i).unwrap() == !(i & 1 == 0));
    }

    let _ = fs::remove_file(&path);
  }
}