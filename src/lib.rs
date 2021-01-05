use std::convert::TryInto;

// ZStd magic number.
const MAGIC_NUMBER: u32 = 0xFD2FB528;
// Hardcoded 16 KB For Encoding
const WINDOW_SIZE: u16 = 16_000;

enum BlockType {
    RawBlock,
    RleBlock,
    ZstdBlock,
}

pub enum Error {
    /// Magic number does not match
    MagicNumber,
    /// Invalid values in the frame header descriptor.
    FrameHeaderDesc,
    /// Window size is too large or too small.
    WindowSize,
    /// There were no blocks in the frame.
    NoBlocks,
    /// Block type is not a valid block type (reserved value used).
    InvalidBlockType,
}

type Result<T> = std::result::Result<T, Error>;

#[inline(always)]
fn decode_u64(input: &[u8]) -> u64 {
    u64::from_le_bytes([input[0], input[1], input[2], input[3],
        input[4], input[5], input[6], input[7],
    ])
}

#[inline(always)]
fn decode_u32(input: &[u8]) -> u32 {
    u32::from_le_bytes([input[0], input[1], input[2], input[3]])
}

#[inline(always)]
fn decode_u24(input: &[u8]) -> u32 {
    u32::from_le_bytes([input[0], input[1], input[2], 0])
}

#[inline(always)]
fn decode_u16(input: &[u8]) -> u16 {
    u16::from_le_bytes([input[0], input[1]])
}

#[inline(always)]
fn decode_u8(input: &[u8]) -> u8 {
    input[0]
}

/// 
pub struct Frame {
    data: Vec<u8>,
}

impl Frame {
    fn encode(self, output: &mut Vec<u8>) {
        ///////////////////// Magic_Number ////////////////////

        let data = &self.data[..];
        output.extend(MAGIC_NUMBER.to_le_bytes().iter());

        ///////////////////// Frame_Header ////////////////////
        
        // Encode frame header descriptor.
        let mut frame_head_desc = 0b0110_0000;
        // 16 bit Frame Content Size
        // Single segment
        // No Checksum
        // No Dictionary
        output.push(frame_head_desc);
        
        ///////////////////// Data_Block(s) ////////////////////
        
        // FIXME
        
        ///////////////////// Content_Checksum ////////////////////

        // FIXME
    }
    
    fn decode(mut input: &[u8]) -> Result<Frame> {
        ///////////////////// Magic_Number ////////////////////
    
        if decode_u32(input) != MAGIC_NUMBER {
            return Err(Error::MagicNumber);
        }
        input = &input[4..];

        ///////////////////// Frame_Header ////////////////////

        // Decode the frame header descriptor.
        let frame_head_desc = decode_u8(input);
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
            return Err(Error::FrameHeaderDesc);
        }
        let content_checksum = content_checksum_flag != 0;
        input = &input[1..];

        // Check for window descriptor if it exists.
        let window_size: Option<u64> = if single_segment_flag == 0 {
            let window_descriptor: u64 = decode_u8(input).into();
            let exponent = (window_descriptor & 0b1111_1000) >> 3;
            let mantissa = window_descriptor & 0b0000_0111;
            let window_log = 10 + exponent;
            let window_base = 1 << window_log;
            let window_add = (window_base / 8) * mantissa;
            input = &input[1..];

            Some(window_base + window_add)
        } else {
            None
        };

        // Check dictionary ID field.
        let dictionary_id: Option<u32> = match dictionary_id_flag {
            0 => None,
            1 => {
                let did = decode_u8(input).into();
                input = &input[1..];
                Some(did)
            },
            2 => {
                let did = decode_u16(input).into();
                input = &input[2..];
                Some(did)
            },
            3 => {
                let did = decode_u32(input);
                input = &input[4..];
                Some(did)
            },
            _ => unreachable!(),
        };

        // Check frame content size.
        let window_size: u64 = if let Some(window_size) = window_size {
            window_size
        } else {
            let window_size: u64 = match fcs_field_size {
                1 => decode_u8(input).into(),
                2 => decode_u16(input).into(),
                4 => decode_u32(input).into(),
                8 => decode_u64(input),
                _ => unreachable!(),
            };
            input = &input[fcs_field_size.into()..];
            window_size
        };

        // Support From 1KB to 8MB
        if window_size > 8_000_000 || window_size < 1_000 {
            return Err(Error::WindowSize);
        }

        // Allocate buffer
        let mut data = Vec::with_capacity(window_size.try_into().unwrap());

        ///////////////////// Data_Block(s) ////////////////////

        // FIXME:

        let block_header = decode_u24(input);
        let mut last_block = (block_header & 1) != 0;
        let mut block_type = match block_header & 0b0110 {
            0b000 => BlockType::RawBlock,
            0b010 => BlockType::RleBlock,
            0b100 => BlockType::ZstdBlock,
            _ => return Err(Error::InvalidBlockType),
        };
        if last_block {
            return Err(Error::NoBlocks);
        }
        let mut block_size = ((block_header >> 3) as usize).min(128_000);
        input = &input[3..];

        loop {
            // Decode this block.
            match block_type {
                BlockType::RawBlock => {
                    // No decompression necessary
                    data.extend(input.iter().cloned().take(block_size));
                    input = &input[block_size..];
                }
                BlockType::RleBlock => {
                    // Run length decompression of a single byte
                    for i in 0..block_size {
                        data.push(input[0]);
                    }
                    input = &input[1..];
                }
                BlockType::ZstdBlock => {
                    // ZStandard decompression
                    
                    // Literals section
                }
            }

            // Check if there are more blocks
            if last_block {
                break;
            }
            let block_header = decode_u24(input);
            last_block = (block_header & 1) != 0;
            block_type = match block_header & 0b0110 {
                0b000 => BlockType::RawBlock,
                0b010 => BlockType::RleBlock,
                0b100 => BlockType::ZstdBlock,
                _ => return Err(Error::InvalidBlockType),
            };
            block_size = ((block_header >> 3) as usize).min(128_000);
            input = &input[3..];
        }

        ///////////////////// Content_Checksum ////////////////////

        // FIXME:

        Ok(Frame { data })
    }
}

/// 
pub struct Encoder {
    
}

impl Encoder {
    pub fn new() -> Self {
        Encoder {
        }
    }
}

/// 
pub struct Decoder {
    
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
