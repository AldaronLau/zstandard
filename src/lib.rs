//! ZStandard compression format encoder and decoder implemented in pure Rust
//! without unsafe.

// Reference: https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#frame_header

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/facebook/zstd/dev/doc/images/zstd_logo86.png",
    html_favicon_url = "https://raw.githubusercontent.com/facebook/zstd/dev/doc/images/zstd_logo86.png",
    html_root_url = "https://docs.rs/zstandard"
)]

// FIXME
#![allow(unreachable_code)]

use std::convert::TryInto;
use std::error::Error;
use std::io::{Read, Write, Error as IoErr, ErrorKind as Kind};
use std::fmt::{Display, Formatter, Error as FmtError};

mod encoder;
mod decoder;
mod parser;

pub use decoder::Decoder;
pub use encoder::Encoder;
use parser::LeDecoder;

/*
 *
 *
*/

fn huffman_stream(stream: &[u8]) {
    let mut stream = stream.iter();
    while let Some(byte) = stream.next_back() {
        
    }
}

// ZStd magic number.
const MAGIC_NUMBER: u32 = 0xFD2FB528;

#[derive(PartialEq)]
enum BlockType {
    RawBlock,
    RleBlock,
    ZstdBlock,
}

#[derive(PartialEq)]
enum LiteralType {
    Raw,
    Rle,
    HuffmanTree,
    HuffmanTreeless,
}

/// Decoder Error.
#[derive(Debug)]
enum DecError {
    MagicNumber,
    FrameHeaderDesc,
    WindowSize,
    NoBlocks,
    InvalidBlockType,
}

impl Display for DecError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        use DecError::*;
        let message = match self {
            MagicNumber => "Magic number does not match",
            FrameHeaderDesc => "Invalid values in the frame header descriptor.",
            WindowSize => "Window size is too large or too small.",
            NoBlocks => "There were no blocks in the frame.",
            InvalidBlockType => "Block type is invalid (reserved value used).",
        };
        write!(f, "{}", message)
    }
}

impl Error for DecError {
}

impl From<DecError> for IoErr {
    fn from(dec_error: DecError) -> IoErr {
        IoErr::new(Kind::InvalidInput, dec_error)
    }
}

#[derive(Default)]
struct Frame {
    data: Vec<u8>,
}

impl Frame {
    fn encode<W: Write>(&mut self, writer: &mut W) -> Result<(), IoErr> {
        ///////////////////// Magic_Number ////////////////////

        let data = &self.data[..];
        writer.write_all(&MAGIC_NUMBER.to_le_bytes())?;

        ///////////////////// Frame_Header ////////////////////
        
        // Encode frame header descriptor.
        let mut frame_head_desc = 0b0110_0000;
        // 16 bit Frame Content Size
        // Single segment
        // No Checksum
        // No Dictionary
        writer.write_all(&[frame_head_desc])?;
        
        ///////////////////// Data_Block(s) ////////////////////
        
        todo!();
        
        ///////////////////// Content_Checksum ////////////////////

        todo!();

        self.data.clear();
        Ok(())
    }
    
    fn decode<R: Read>(&mut self, reader: &mut R) -> Result<(), IoErr> {
        let mut dec = LeDecoder::new(reader);

        ///////////////////// Magic_Number ////////////////////

        if dec.u32()? != MAGIC_NUMBER {
            Err(DecError::MagicNumber)?
        }

        ///////////////////// Frame_Header ////////////////////

        // Decode the frame header descriptor.
        let frame_head_desc = dec.u8()?;
        let frame_content_size_flag = frame_head_desc & 0b1100_0000;
        let single_segment_flag = frame_head_desc & 0b0010_0000;
        let unused_reserved_bits = frame_head_desc & 0b0001_1000;
        let content_checksum_flag = frame_head_desc & 0b0000_0100;
        let dictionary_id_flag = frame_head_desc & 0b0000_0011;
        // Interpret frame header descriptor.
        let fcs_field_size = match frame_content_size_flag {
            0b0000_0000 => single_segment_flag >> 5,
            0b0100_0000 => 2,
            0b1000_0000 => 4,
            0b1100_0000 => 8,
            _ => unreachable!(),
        };
        if unused_reserved_bits != 0 {
            Err(DecError::FrameHeaderDesc)?
        }
        let content_checksum = content_checksum_flag != 0;

        // Check for window descriptor if it exists.
        let window_size: Option<u64> = if single_segment_flag == 0 {
            let window_descriptor: u64 = dec.u8()?.into();
            let exponent = (window_descriptor & 0b1111_1000) >> 3;
            let mantissa = window_descriptor & 0b0000_0111;
            let window_log = 10 + exponent;
            let window_base = 1 << window_log;
            let window_add = (window_base / 8) * mantissa;

            Some(window_base + window_add)
        } else {
            None
        };

        // Check dictionary ID field.
        let dictionary_id: Option<u32> = match dictionary_id_flag {
            0 => None,
            1 => {
                let did = dec.u8()?.into();
                Some(did)
            },
            2 => {
                let did = dec.u16()?.into();
                Some(did)
            },
            3 => {
                let did = dec.u32()?;
                Some(did)
            },
            _ => unreachable!(),
        };

        // Check frame content size.
        let window_size: u64 = if let Some(window_size) = window_size {
            window_size
        } else {
            let window_size: u64 = match fcs_field_size {
                1 => dec.u8()?.into(),
                2 => dec.u16()?.into(),
                4 => dec.u32()?.into(),
                8 => dec.u64()?,
                _ => unreachable!(),
            };
            window_size
        };

        // Support From 1KB to 8MB
        if window_size > 8_000_000 || window_size < 1_000 {
            Err(DecError::WindowSize)?
        }

        // Resize buffer (to be overwritten)
        self.data.resize(window_size.try_into().unwrap(), 0);

        ///////////////////// Data_Block(s) ////////////////////

        // FIXME:

        let block_header = dec.u24()?;
        let mut last_block = (block_header & 1) != 0;
        let mut block_type = match block_header & 0b0110 {
            0b000 => BlockType::RawBlock,
            0b010 => BlockType::RleBlock,
            0b100 => BlockType::ZstdBlock,
            _ => Err(DecError::InvalidBlockType)?,
        };
        if last_block {
            Err(DecError::NoBlocks)?
        }
        let mut block_size = ((block_header >> 3) as usize).min(128_000);
        let mut buf = &mut self.data[..];

        loop {
            // Decode this block.
            match block_type {
                BlockType::RawBlock => {
                    // No decompression necessary
                    dec.bytes(&mut buf[..block_size])?;
                    buf = &mut buf[block_size..];
                }
                BlockType::RleBlock => {
                    // Run length decompression of a single byte
                    let single_byte = dec.u8()?;
                    for i in &mut buf[..block_size] {
                        *i = single_byte;
                    }
                    buf = &mut buf[block_size..];
                }
                BlockType::ZstdBlock => {
                    // ZStandard decompression
                    
                    //////////// Literals section //////////
                    
                    // Literals Section header
                    let first_nibble = dec.u(4, 0)?;
                    let literal_type = match first_nibble & 0b0011 {
                        0b00 => LiteralType::Raw,
                        0b01 => LiteralType::Rle,
                        0b10 => LiteralType::HuffmanTree,
                        0b11 => LiteralType::HuffmanTreeless,
                        _ => unreachable!(),
                    };
                    use LiteralType::*;
                    let (regenerated_size, compressed_size, four_huffman_streams) = match literal_type {
                        Rle | Raw => {
                            // Size format uses 1 or 2 bits.
                            let rs = match first_nibble & 0b1100 {
                                // 1 Bit (Regenerated Size: u5)
                                0b0000 | 0b1000 => dec.u(5, 5)?,
                                // 2 Bit (Regenerated Size: u12)
                                0b0100 => dec.u(12, 4)?,
                                // 2 Bit (Regenerated Size: u20)
                                0b1100 => dec.u(20, 4)?,

                                _ => unreachable!(),
                            };
                            (rs, None, false)
                        }
                        HuffmanTree | HuffmanTreeless => {
                            // Size format always uses 2 bits.
                            let (rs, cs, fh) = match first_nibble & 0b1100 {
                                // 3 Byte Header
                                // Single Stream: Regenerated Size: u10
                                0b0000 => (dec.u(10, 4)?, dec.u(10, 2)?, false),
                                // 4 Streams: Regenerated Size: u10
                                0b0100 => (dec.u(10, 4)?, dec.u(10, 2)?, true),

                                // 4 Byte Header
                                // 4 Streams: Regenerated Size: u14
                                0b1000 => (dec.u(14, 4)?, dec.u(14, 6)?, true),

                                // 5 Byte Header
                                // 4 Streams: Regenerated Size: u18
                                0b1100 => (dec.u(18, 4)?, dec.u(18, 2)?, true),

                                _ => unreachable!(),
                            };
                            (rs, Some(cs), fh)
                        }
                    };
                    
                    // Huffman tree description
                    if literal_type == LiteralType::HuffmanTree {
                        todo!();
                    }

                    // Jump Table: compressed sizes of first three streams.
                    let jump_table = if four_huffman_streams {
                        Some([dec.u16()?, dec.u16()?, dec.u16()?])
                    } else {
                        None
                    };

                    // Decode Streams
                    if let Some(compressed_size) = compressed_size {
                        let mut streams = vec![0; compressed_size as usize];
                        dec.bytes(&mut streams)?;
                        if let Some(jump_table) = jump_table {
                            println!("{}, {}", jump_table[0], streams.len());
                            huffman_stream(&streams[..jump_table[0].into()]);
                            huffman_stream(&streams[..jump_table[1].into()]);
                            huffman_stream(&streams[..jump_table[2].into()]);
                            huffman_stream(&streams[..]);
                        } else {
                            huffman_stream(&streams[..]);
                        }
                    }
                    
                    //////////// Sequences section //////////

                    todo!();

                    //////////// **Sequence Execution** //////////// 
                    // Now that we've calculated the literals and sequences, we
                    // can turn it into data.

                    todo!();
                }
            }

            // Check if there are more blocks
            if last_block {
                break;
            }
            let block_header = dec.u24()?;
            last_block = (block_header & 1) != 0;
            block_type = match block_header & 0b0110 {
                0b000 => BlockType::RawBlock,
                0b010 => BlockType::RleBlock,
                0b100 => BlockType::ZstdBlock,
                _ => Err(DecError::InvalidBlockType)?,
            };
            block_size = ((block_header >> 3) as usize).min(128_000);
        }

        ///////////////////// Content_Checksum ////////////////////

        // FIXME:

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
