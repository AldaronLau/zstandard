use std::io::{Read, Error as IoError};

use super::Frame;

/// A ZStandard Stream Decoder that implements [`Read`](std::io::Read).
pub struct Decoder<R: Read>(R, Frame, usize);

impl<R: Read> Decoder<R> {
    /// Create a new ZStandard stream decoder that reads from a `Read`er.
    pub fn new(reader: R) -> Self {
        Self(reader, Frame::default(), 0)
    }
}

impl<R: Read> Read for Decoder<R> {
    fn read(&mut self, mut buf: &mut [u8]) -> Result<usize, IoError> {
        // Entire length of the buffer.
        let orig_len = buf.len();
        // Fill up the buffer until there are no remaining bytes.
        while !buf.is_empty() {
            // Check if there is no frame data, then decode next frame.
            if self.1.data[self.2..].is_empty() {
                // Try to decode the next frame.
                self.1.decode(&mut self.0)?;
                // If still empty, return early with partially filled buffer.
                if self.1.data[self.2..].is_empty() {
                    return Ok(orig_len - buf.len());
                }
            }
            // Get the frame data
            let data = &self.1.data[self.2..];
            // Check for number of bytes from previous frame.
            let amt_to_copy = data.len().min(buf.len());
            // Copy bytes
            for i in 0..amt_to_copy {
                buf[i] = data[i];
            }
            // Move buffer index.
            self.2 += amt_to_copy;
            // Shrink writeable slice of out buffer.
            buf = &mut buf[amt_to_copy..];
        }
        // Successfully filled up entire buffer.
        Ok(orig_len)
    }
}
