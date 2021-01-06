use std::io::Read;

use zstandard::Decoder;

fn main() -> std::io::Result<()> {
    // Open ZST file.
    let zst = std::fs::File::open("./testfiles/z000000.zst")?;
    // Decoding stream
    let mut decoder = Decoder::new(zst);
    let mut decoded = Vec::new();
    // Read into decoded.
    decoder.read_to_end(&mut decoded)?;
    
    // Open original data file.
    let orig = std::fs::read("./testfiles/z000000")?;

    // Compare
    assert_eq!(orig, decoded);

    Ok(())
}
