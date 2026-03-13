use anyhow::{bail, Context, Result};

/// Represents a single delta operation - either Copy or Insert
#[derive(Debug)]
enum DeltaOp {
    /// Copy operation that references data from the base object
    Copy {
        /// Offset in base object to copy from
        offset: usize,
        /// Number of bytes to copy
        length: usize,
    },
    /// Insert operation that adds new data
    Insert(Vec<u8>),
}

/// The delta format is a compact binary format that encodes differences between objects
/// It consists of:
///   1. Header with base and result sizes
///   2. Series of COPY/INSERT operations
pub struct Delta<'a> {
    /// The raw delta data being parsed
    data: &'a [u8],
    /// Current read position within the data
    position: usize,
}

impl<'a> Delta<'a> {
    /// Creates a new Delta from raw bytes
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    /// -------------------------------------------------------------------
    /// DELTA HEADER PARSING
    /// -------------------------------------------------------------------
    /// Header format:
    ///   [base_size][result_size]
    ///
    /// Where sizes are variable-length integers (see read_size())
    ///
    pub fn parse_header(&mut self) -> Result<(usize, usize)> {
        let base_size = self.read_size()?;
        let result_size = self.read_size()?;
        Ok((base_size, result_size))
    }

    /// -------------------------------------------------------------------
    /// OPERATION PARSING
    /// -------------------------------------------------------------------
    /// Delta operations come in two types:
    ///   1. COPY - references data from base object
    ///   2. INSERT - adds new data
    ///
    pub fn parse_ops(&mut self) -> Result<Vec<DeltaOp>> {
        let mut ops = Vec::new();

        while self.position < self.data.len() {
            let cmd = self.read_byte()?;

            // determines operation type
            if cmd & 0x80 != 0 {
                // indicates a Copy operation
                let (offset, length) = self.parse_copy_op(cmd)?;
                ops.push(DeltaOp::Copy { offset, length });
            } else {
                // Otherwise it's an Insert operation
                let data = self.parse_insert_data(cmd)?;
                ops.push(DeltaOp::Insert(data));
            }
        }

        Ok(ops)
    }

    /// Parses a Copy operation from the command byte
    ///
    /// # Arguments
    /// * `cmd` - The command byte
    ///
    /// # Returns
    /// Tuple of (offset, length) for the copy operation
    ///
    /// # Errors
    /// Returns error if the offset/length encoding is invalid
    ///
    pub fn parse_copy_op(&mut self, cmd: u8) -> Result<(usize, usize)> {
        let mut offset = 0;
        let mut length = 0;

        // ----------------------------------------------------------------------------
        // PARSE OFFSET (bits 0-3 of cmd byte)
        // ----------------------------------------------------------------------------
        // The command byte uses bits 0-3 as flags indicating which offset bytes follow:
        // - Bit 0 (0x01): if set, offset byte 0 exists
        // - Bit 1 (0x02): if set, offset byte 1 exists
        // - Bit 2 (0x04): if set, offset byte 2 exists
        // - Bit 3 (0x08): if set, offset byte 3 exists
        //
        // Each existing byte contributes 8 bits to the final offset value
        // in little-endian order (first byte is least significant)
        for i in 0..4 {
            // Check if bit 'i' is set in the command byte
            if cmd & (1 << i) != 0 {
                // Read the next byte and incorporate it into the offset
                // Shift it left by (i * 8) bits to place it in correct position
                // Example:
                // - If i=0 (first byte), no shift (least significant byte)
                // - If i=1, shift left 8 bits (second byte)
                // - etc.
                offset |= (self.read_byte()? as usize) << (i * 8);
            }
        }

        // ------------------------------------------------------------------------
        // PARSE LENGTH (bits 4-6 of cmd byte)
        // ------------------------------------------------------------------------
        // The command byte uses bits 4-6 as flags indicating which length bytes follow:
        // - Bit 4 (0x10): if set, length byte 0 exists
        // - Bit 5 (0x20): if set, length byte 1 exists
        // - Bit 6 (0x40): if set, length byte 2 exists
        //
        // Each existing byte contributes 8 bits to the final length value
        // in little-endian order (first byte is least significant)
        for i in 4..7 {
            // Check if bit 'i' is set in the command byte
            if cmd & (1 << i) != 0 {
                // Read the next byte and incorporate it into the length
                // Shift it left by ((i-4) * 8) bits to normalize position
                // (since we're using bits 4-6 but want to treat them as 0-2)
                // Example:
                // - If i=4, shift left 0 bits (length byte 0)
                // - If i=5, shift left 8 bits (length byte 1)
                // - If i=6, shift left 16 bits (length byte 2)
                length |= (self.read_byte()? as usize) << ((i - 4) * 8);
            }
        }

        // ------------------------------------------------------------------------
        // SPECIAL CASE HANDLING
        // ------------------------------------------------------------------------
        // In Git's delta format, a length of 0 is special and means 64KB (0x10000)
        // This allows representing large copies efficiently
        //
        if length == 0 {
            length = 0x10000; // 64KB
        }

        Ok((offset, length))
    }

    /// -------------------------------------------------------------------
    /// INSERT OPERATION PARSING
    /// -------------------------------------------------------------------
    /// Format:
    /// [LENGTH][DATA...]
    /// Where:
    /// - LENGTH is a 1-byte value (0 <= LENGTH <= 127)
    /// - Followed by exactly LENGTH bytes of data
    ///
    /// # Arguments
    /// * `cmd` - The command byte (length of data to insert)
    ///
    pub fn parse_insert_data(&mut self, cmd: u8) -> Result<Vec<u8>> {
        let length = cmd as usize;
        let start = self.position;
        let end = start + length;

        if end > self.data.len() {
            bail!("Insert operation overflows delta data");
        }

        self.position = end;
        Ok(self.data[start..end].to_vec())
    }

    /// Reads a single byte from the delta data
    ///
    /// # Errors
    /// Returns error if we've reached end of data
    ///
    pub fn read_byte(&mut self) -> Result<u8> {
        if self.position >= self.data.len() {
            bail!("Unexpected end of delta");
        }
        let b = self.data[self.position];
        self.position += 1;
        Ok(b)
    }

    /// Reads a variable-length size encoding
    ///
    /// # Returns
    /// The decoded size value
    ///
    pub fn read_size(&mut self) -> Result<usize> {
        let mut size = 0;
        let mut shift = 0;
        loop {
            let byte = self.read_byte()?;
            size |= ((byte & 0x7F) as usize) << shift;
            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift >= usize::BITS {
                bail!("Variable-length size exceeds maximum for usize");
            }
        }
        Ok(size)
    }
}

/// -------------------------------------------------------------------
/// DELTA APPLICATION
/// -------------------------------------------------------------------
/// Reconstructs target object by applying delta to base object
///
/// # Arguments
/// * `base` - The base object data
/// * `delta` - The delta data to apply
///
/// # Returns
/// The reconstructed target object data
///
/// # Errors
/// Returns error if:
/// - Base size doesn't match delta header
/// - Any operation is invalid
/// - Result size doesn't match delta header
///
pub fn apply_delta(base: &[u8], delta: &[u8]) -> Result<Vec<u8>> {
    let mut parser = Delta::new(delta);

    // 1: Parse header and validate
    let (base_size, result_size) = parser
        .parse_header()
        .context("Failed to parse delta header")?;
    if base.len() != base_size {
        bail!(
            "Base size mismatch (expected {}, got {})",
            base_size,
            base.len()
        );
    }

    // 2: Parse all operations
    let ops = parser
        .parse_ops()
        .context("Failed to parse delta opearations")?;

    // 3: Apply operations
    let mut result = Vec::with_capacity(result_size);
    for op in ops {
        match op {
            DeltaOp::Copy { offset, length } => {
                let end = offset.checked_add(length).context("Copy length overflow")?;

                if offset >= base.len() || end > base.len() {
                    bail!(
                        "Copy range {}-{} exceeds base size {}",
                        offset,
                        end,
                        base.len()
                    );
                }
                result.extend_from_slice(&base[offset..end]);
            }
            DeltaOp::Insert(data) => {
                result.extend(data);
            }
        }
    }

    // 4: Verify final size matches header
    if result.len() != result_size {
        bail!(
            "Result size mismatch (header expected {} bytes, got {} bytes in reconstructed data)",
            result_size,
            result.len()
        );
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_parse_header() {
        let data = [0x01, 0x01]; // base = 1, result = 1
        let mut parser = Delta::new(&data);
        assert_eq!(parser.parse_header().unwrap(), (1, 1));

        let data = [0x80, 0x01, 0x80, 0x01];

        // Size encoding:
        // Byte:
        // ------|-------|--------
        // Byte 0: 1     | Value0 (7 bits)
        // Byte 1: 1     | Value1 (next 7 bits)
        // ...
        // Last Byte: 0 | ValueN (7 bits)
        // Size = Value0 + (Value1 << 7) + (Value2 << 14) + ... + (ValueN << (7*N))

        // So data = [0x80, 0x01, 0x80, 0x01] represents base_size = 128, result_size = 128

        let mut parser = Delta::new(&data);
        assert_eq!(parser.parse_header().unwrap(), (128, 128));

        // Size 0xFFFFFFFF (4294967295)
        // Need 5 bytes:
        // 0xFF & 0x7F = 0x7F
        // (0xFF & 0x7F) << 7 = 0x3F80
        // (0xFF & 0x7F) << 14 = 0x1FC000
        // (0xFF & 0x7F) << 21 = 0xFE00000
        // (0x0F & 0x7F) << 28 = 0x780000000 -> Error in manual calculation, needs check with code
        // byte = 0xFF (MSB set, value 0x7F), size |= 0x7F << 0
        // byte = 0xFF (MSB set, value 0x7F), size |= 0x7F << 7
        // byte = 0xFF (MSB set, value 0x7F), size |= 0x7F << 14
        // byte = 0xFF (MSB set, value 0x7F), size |= 0x7F << 21
        // byte = 0x0F (MSB not set, value 0x0F), size |= 0x0F << 28, break.
        // Size = 0x7F + (0x7F << 7) + (0x7F << 14) + (0x7F << 21) + (0x0F << 28)
        // This adds up to 0xFFFFFFFF. So the data [0xFF; 4] ++ [0x0F] should represent 0xFFFFFFFF

        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F]; // base = 0xFFFFFFFF, result = 0xFFFFFFFF
        let mut parser = Delta::new(&data);
        assert_eq!(
            parser.parse_header().unwrap(),
            (0xFFFFFFFF as usize, 0xFFFFFFFF as usize)
        );
    }

    #[test]
    fn test_parse_copy() {
        use DeltaOp::Copy;

        // cmd = 0x90 = 1001_0000 (Copy, Length bit 4) -> offset flags 0x00 (none), length flags 0x10 (byte 0)
        // Next byte = 0x05 -> length 5, offset 0
        //
        let data = [0x90, 0x05];
        let mut parser = Delta::new(&data);
        let ops = parser.parse_ops().unwrap();
        assert_eq!(ops.len(), 1);
        assert_matches!(
            ops[0],
            Copy {
                offset: 0,
                length: 5
            }
        );

        // cmd = 0x70 = 0111_0000 (Copy, Length bits 4,5,6) -> offset flags 0x00 (none), length flags 0x70 (bytes 0,1,2)
        // Next 3 bytes encode length. if those bytes were 0x00 0x00 0x00 -> length 0 -> special case 64KB
        let data = [0xF0, 0x00, 0x00, 0x00]; // cmd = 0xF0 = 1111_0000 (Copy, offset none, length bits 4,5,6)
                                             // next 3 bytes 0,0,0 -> length 0. Special case = 64KB. offset 0

        let mut parser = Delta::new(&data);
        let ops = parser.parse_ops().unwrap();
        assert_eq!(ops.len(), 1);
        assert_matches!(
            ops[0],
            Copy {
                offset: 0,
                length: 0x10000
            }
        ); // length 65536

        // cmd = 0x91 = 1001_0001 (Copy, Offset bit 0, Length bit 4)
        // Next byte O0 = 0x06 -> offset = 6
        // Next byte L0 = 0x06 -> length = 6

        let data = [0x91, 0x06, 0x06]; // cmd = 0x91, offset = 6 (byte 0), length = 6 (byte 0)
        let mut parser = Delta::new(&data);
        let ops = parser.parse_ops().unwrap();
        assert_eq!(ops.len(), 1);
        assert_matches!(
            ops[0],
            Copy {
                offset: 6,
                length: 6
            }
        );

        // cmd = 0xFF = 1111_1111 (Copy, Offset bits 0-3, Length bits 4-6)
        // O0..O3 = 01 00 00 00 -> offset = 1 (bytes follow in little-endian order)
        // L0..L2 = 80 02 00 -> length = 0x80 + (0x02 << 8) = 0x80 + 0x200 = 0x280
        // Data: FF 01 00 00 00 80 02 00
        // Indices:  0  1  2  3  4  5  6  7

        let data = [0xFF, 0x01, 0x00, 0x00, 0x00, 0x80, 0x02, 0x00];
        let mut parser = Delta::new(&data);
        let ops = parser.parse_ops().unwrap();
        assert_eq!(ops.len(), 1);
        assert_matches!(
            ops[0],
            Copy {
                offset: 0x00000001, // O0 = 1
                length: 0x00000280  // L0 = 0x80, L1 = 0x02, L2 = 0x00
            }
        );
    }

    #[test]
    fn test_parse_insert() {
        use DeltaOp::Insert;

        // Length = 3, data = 'abc'
        let data = [0x03, b'a', b'b', b'c']; // cmd = 3 (Insert length 3)
        let mut parser = Delta::new(&data);
        let ops = parser.parse_ops().unwrap();
        assert_eq!(ops.len(), 1);

        // Fix: Use if let to match and bind, then assert equality on the data
        if let Insert(d) = &ops[0] {
            assert_eq!(d, b"abc");
        } else {
            panic!("Expected Insert operation");
        }
        // Or, using assert_matches with guard:
        // assert_matches!(ops[0], Insert(d) if d == b"abc");

        // Length = 2, data = 'ab', then Length = 3, data = 'cde'
        let data = [0x02, b'a', b'b', 0x03, b'c', b'd', b'e'];
        let mut parser = Delta::new(&data);
        let ops = parser.parse_ops().unwrap();
        assert_eq!(ops.len(), 2);

        if let Insert(d1) = &ops[0] {
            assert_eq!(d1, b"ab");
        } else {
            panic!("Expected Insert operation at index 0");
        }

        if let Insert(d2) = &ops[1] {
            assert_eq!(d2, b"cde");
        } else {
            panic!("Expected Insert operation at index 1");
        }
        // Or:
        // assert_matches!(ops[0], Insert(d) if d == b"ab");
        // assert_matches!(ops[1], Insert(d) if d == b"cde");
    }

    #[test]
    fn test_apply_delta() {
        // Base and Result are identical
        let base = b"Hello world";

        // Delta: base_size = 11 (0x0B), result_size = 11 (0x0B)
        // Op: COPY offset = 0, length = 11. Cmd = 0x80 | offset_flags = 0x00 | length_flags = 0x10 = 0x90
        // Bytes for op: 0x90, L0 = 11 (0x0B)
        let delta = [
            0x0B, 0x0B, // base_size = 11, result_size = 11
            0x90, 0x0B, // COPY offset = 0 (no flags), length = 11 (flag 4, byte 0x0B)
        ];

        let result = apply_delta(base, &delta).unwrap();
        assert_eq!(result, b"Hello world");

        // Base = "Hello", insert " w!" at the end. Result = "Hello w!"
        let base = b"Hello";
        // Delta: base_size = 5 (0x05), result_size = 8 (0x08)
        // Op 1: COPY offset = 0, length = 5. Cmd = 0x90, L0 = 5 (0x05) -> [0x90, 0x05]
        // Op 2: INSERT " w!". Cmd = 3 (length 3). Data: [0x20, 0x77, 0x21] -> [0x03, 0x20, 0x77, 0x21]

        let delta = [
            0x05, 0x08, // base_size = 5, result_size = 8
            0x90, 0x05, // COPY offset = 0, length = 5
            0x03, b' ', b'w', b'!', // INSERT length = 3, data = " w!"
        ];
        let result = apply_delta(base, &delta).unwrap();
        assert_eq!(result, b"Hello w!"); // Fixed expected result

        // Copy two parts from base
        let base = b"Hello world, welcome to orf"; // len 26

        // Delta: base_size=26 (0x1A), result_size=12 (0x0C)
        // Op 1: COPY "Hello " (offset 0, len 6). Cmd=0x90, L0=6 (0x06) -> [0x90, 0x06]
        // Op 2: COPY "world," (offset 6, len 6). Cmd=0x91 (offset bit 0, length bit 4), O0=6 (0x06), L0=6 (0x06) -> [0x91, 0x06, 0x06]

        let delta = [
            0x1B, 0x0C, // base_size = 26, result_size = 12
            0x90, 0x06, // COPY offset = 0, length = 6
            0x91, 0x06, 0x06, // COPY offset = 6, length = 6
        ];
        let result = apply_delta(base, &delta).unwrap();
        assert_eq!(result, b"Hello world,");
    }
}
