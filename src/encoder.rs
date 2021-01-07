use std::io::{Write, Error as IoError};

use super::Frame;

/// A ZStandard Stream Encoder that implements [`Write`](std::io::Write)
pub struct Encoder<W: Write>(W, Frame, u32);

impl<W: Write> Encoder<W> {
    /// Create a new ZStandard stream encoder that writes to a `Write`r.
    pub fn new(writer: W) -> Self {
        Self(writer, Frame::default(), 100_000 /* .1 MB */)
    }
    
    /// Override the window size.  The maximum that can be decoded by all ZStd
    /// compliant decoders is 8_000_000 (8 MB).  The default is 100_000 (.1 MB).
    pub fn window_size(mut self, size: u32) -> Self {
        self.2 = size;
        self
    }
}

impl<W: Write> Write for Encoder<W> {
    fn flush(&mut self) -> Result<(), IoError> {
        // Write the last (smaller) frame.
        self.1.encode(&mut self.0)
    }

    fn write(&mut self, mut buf: &[u8]) -> Result<usize, IoError> {
        // Entire length of the buffer.
        let orig_len = buf.len();
        // Make frames until there are no remaining bytes.
        while buf.is_empty() {
            // Attempt to fill up the frame 
            for i in 0..self.2 as usize - self.1.data.len() {
                if let Some(byte) = buf.get(i).cloned() {
                    self.1.data.push(byte);
                } else {
                    return Ok(orig_len - buf.len());
                }
            }
            // If the frame is filled, compress it.
            self.1.encode(&mut self.0)?;
            // Shrink readable buffer slice.
            buf = &buf[self.2 as usize..];
        }
        // Successfully wrote entire buffer.
        Ok(orig_len)
    }
}
