//! 

// TODO: Separate out into a library that can be additionally used by png_pong
#![allow(unused)]

use std::io::{Result, Read};
use std::ops::{BitOrAssign, Shl};

/// A little endian decoder.
pub struct LeDecoder<R: Read>(R, [u8; 1]);

impl<R: Read> LeDecoder<R> {
    /// Create a new little endian stream decoder that reader from a `Read`er.
    #[inline(always)]
    pub fn new(reader: R) -> Self {
        Self(reader, [0])
    }

    /// Unaligned read of arbitrary bit length up to 32.
    #[inline(always)]
    pub fn u(&mut self, bits: u8, mut leftover: u8) -> Result<u32> {
        // Write leftover bits (most significant in least significant place).
        let mut output = if leftover != 0 {
            self.1[0] as u32 >> (8 - leftover)
        } else { 0 };

        // Write bytes with full cover.
        let rest = bits - leftover;
        for i in 0..rest >> 3 {
            self.0.read_exact(&mut self.1)?;
            output |= (self.1[0] as u32) << leftover;
            leftover += 8;
        }

        // Write portion of next least significant bits to the right in more
        // significant place).
        let rest = bits - leftover;
        if rest != 0 {
            self.0.read_exact(&mut self.1)?;
            output |= (((self.1[0] << (8 - rest)) >> (8 - rest)) as u32) << leftover;
        }

        // Convert to Little Endian.
        Ok(u32::from_le(output))
    }

    /// Aligned read of u8.
    #[inline(always)]
    pub fn u8(&mut self) -> Result<u8> {
        let mut buf = [0; 1];
        self.0.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Aligned read of u16.
    #[inline(always)]
    pub fn u16(&mut self) -> Result<u16> {
        let mut buf = [0; 2];
        self.0.read_exact(&mut buf)?;
        Ok(aligned_le(&buf))
    }

    /// Aligned read of u24.
    #[inline(always)]
    pub fn u24(&mut self) -> Result<u32> {
        let mut buf = [0; 3];
        self.0.read_exact(&mut buf)?;
        Ok(aligned_le(&buf))
    }

    /// Aligned read of u32.
    #[inline(always)]
    pub fn u32(&mut self) -> Result<u32> {
        let mut buf = [0; 4];
        self.0.read_exact(&mut buf)?;
        Ok(aligned_le(&buf))
    }

    /// Aligned read of u48.
    #[inline(always)]
    pub fn u48(&mut self) -> Result<u64> {
        let mut buf = [0; 6];
        self.0.read_exact(&mut buf)?;
        Ok(aligned_le(&buf))
    }

    /// Aligned read of u64.
    #[inline(always)]
    pub fn u64(&mut self) -> Result<u64> {
        let mut buf = [0; 8];
        self.0.read_exact(&mut buf)?;
        Ok(aligned_le(&buf))
    }

    /// Aligned read of u128.
    #[inline(always)]
    pub fn u128(&mut self) -> Result<u128> {
        let mut buf = [0; 16];
        self.0.read_exact(&mut buf)?;
        Ok(aligned_le(&buf))
    }
    
    /// Aligned read of some number of bytes.
    #[inline(always)]
    pub fn bytes(&mut self, buf: &mut [u8]) -> Result<()> {
        self.0.read_exact(buf)
    }
}

/// Do an aligned read of a little endian integer.
#[inline(always)]
fn aligned_le<T: From<u8> + BitOrAssign + Shl<usize, Output = T>>(buf: &[u8]) -> T {
    let mut output: T =  0.into();
    for (i, b) in buf.iter().map(|v| -> T { (*v).into() }).enumerate() {
        output |= b << (i << 3);
    }
    output
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::LeDecoder;

    #[test]
    fn decode_32() {
        let mut reader = Cursor::new(&[10, 0, 0, 0]);
        let mut dec = LeDecoder::new(&mut reader);

        assert_eq!(dec.u32().unwrap(), 10);
    }

    #[test]
    fn decode_arbitrary() {
        // 4 bit, 8 bit LE, 4 bit
        let mut reader = Cursor::new(&[0b1100_0010, 0b0011_1101]);
        let mut dec = LeDecoder::new(&mut reader);
        
        let a = dec.u(4, 0).unwrap();
        let b = dec.u(8, 4).unwrap();
        let c = dec.u(4, 4).unwrap();

        assert_eq!(0b0010, a);
        assert_eq!(0b1101_1100, b);
        assert_eq!(0b0011, c);
    }
}
