//! NNUE weight storage, .fnnue binary format, and random weight generation.
//!
//! The `.fnnue` format is a simple header + flat i16 weight arrays, little-endian.
//! This is the canonical format shared between Rust inference and Python training.
//!
//! Stage 16: NNUE Architecture + Training Pipeline

use super::features::{HIDDEN1_SIZE, HIDDEN2_SIZE, TOTAL_FEATURES};
use std::io::{self, Read, Write};

// ─── Format Constants ──────────────────────────────────────────────────────

/// Magic bytes identifying a .fnnue file.
const FNNUE_MAGIC: [u8; 8] = *b"FNNUE\x00\x01\x00";

/// Header size in bytes.
#[cfg(test)]
const HEADER_SIZE: usize = 28;

// ─── Weight Struct ─────────────────────────────────────────────────────────

/// NNUE network weights.
///
/// Feature weights are boxed (~2.3 MB) to avoid stack overflow.
/// All other weight arrays fit on the stack.
pub struct NnueWeights {
    /// Feature → hidden1 weights. Shape: [TOTAL_FEATURES][HIDDEN1_SIZE].
    pub feature_weights: Box<[[i16; HIDDEN1_SIZE]; TOTAL_FEATURES]>,
    /// Feature layer biases. Shape: [HIDDEN1_SIZE].
    pub feature_biases: [i16; HIDDEN1_SIZE],
    /// Hidden1 → hidden2 weights. Shape: [HIDDEN1_SIZE][HIDDEN2_SIZE].
    pub hidden1_weights: [[i16; HIDDEN2_SIZE]; HIDDEN1_SIZE],
    /// Hidden1 biases. Shape: [HIDDEN2_SIZE].
    pub hidden1_biases: [i16; HIDDEN2_SIZE],
    /// Hidden2 → output weights. Shape: [HIDDEN2_SIZE].
    pub hidden2_weights: [i16; HIDDEN2_SIZE],
    /// Hidden2 bias (scalar).
    pub hidden2_bias: i16,
}

impl NnueWeights {
    /// Create zero-initialized weights.
    ///
    /// Feature weights are allocated directly on the heap (~2.3 MB)
    /// to avoid stack overflow.
    pub fn zeros() -> Self {
        // Allocate feature weights on heap without touching the stack.
        // vec![...].into_boxed_slice() then convert to boxed array.
        let feature_weights: Box<[[i16; HIDDEN1_SIZE]; TOTAL_FEATURES]> = {
            let vec: Vec<[i16; HIDDEN1_SIZE]> = vec![[0i16; HIDDEN1_SIZE]; TOTAL_FEATURES];
            // SAFETY: Vec has exactly TOTAL_FEATURES elements, matching the array size.
            let boxed_slice = vec.into_boxed_slice();
            // Convert Box<[[i16; 256]]> to Box<[[i16; 256]; 4488]>
            let ptr = Box::into_raw(boxed_slice) as *mut [[i16; HIDDEN1_SIZE]; TOTAL_FEATURES];
            unsafe { Box::from_raw(ptr) }
        };

        Self {
            feature_weights,
            feature_biases: [0i16; HIDDEN1_SIZE],
            hidden1_weights: [[0i16; HIDDEN2_SIZE]; HIDDEN1_SIZE],
            hidden1_biases: [0i16; HIDDEN2_SIZE],
            hidden2_weights: [0i16; HIDDEN2_SIZE],
            hidden2_bias: 0,
        }
    }

    /// Generate random weights from a seed (for pipeline verification).
    ///
    /// Uses xorshift64 PRNG. Weights are small values suitable for testing.
    pub fn random(seed: u64) -> Self {
        let mut rng = Xorshift64::new(seed);
        let mut weights = Self::zeros();

        // Feature weights: small range to keep accumulator values reasonable
        for row in weights.feature_weights.iter_mut() {
            for w in row.iter_mut() {
                *w = rng.next_i16_range(-32, 32);
            }
        }
        for b in weights.feature_biases.iter_mut() {
            *b = rng.next_i16_range(-16, 16);
        }

        // Hidden1 weights
        for row in weights.hidden1_weights.iter_mut() {
            for w in row.iter_mut() {
                *w = rng.next_i16_range(-64, 64);
            }
        }
        for b in weights.hidden1_biases.iter_mut() {
            *b = rng.next_i16_range(-16, 16);
        }

        // Hidden2 weights
        for w in weights.hidden2_weights.iter_mut() {
            *w = rng.next_i16_range(-64, 64);
        }
        weights.hidden2_bias = rng.next_i16_range(-16, 16);

        weights
    }

    /// Compute an architecture hash from layer sizes.
    ///
    /// Used to validate that a .fnnue file matches the expected architecture.
    pub fn architecture_hash() -> u32 {
        // Simple hash of layer dimensions
        let mut hash: u32 = 0x46_4E_4E_55; // "FNNU"
        hash = hash.wrapping_mul(31).wrapping_add(TOTAL_FEATURES as u32);
        hash = hash.wrapping_mul(31).wrapping_add(HIDDEN1_SIZE as u32);
        hash = hash.wrapping_mul(31).wrapping_add(HIDDEN2_SIZE as u32);
        hash = hash.wrapping_mul(31).wrapping_add(1); // output size
        hash
    }

    /// Save weights to .fnnue binary format.
    pub fn save<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        // Header
        writer.write_all(&FNNUE_MAGIC)?;
        writer.write_all(&Self::architecture_hash().to_le_bytes())?;
        writer.write_all(&(TOTAL_FEATURES as u32).to_le_bytes())?;
        writer.write_all(&(HIDDEN1_SIZE as u32).to_le_bytes())?;
        writer.write_all(&(HIDDEN2_SIZE as u32).to_le_bytes())?;
        writer.write_all(&(1u32).to_le_bytes())?; // output_size

        // Feature weights: [TOTAL_FEATURES][HIDDEN1_SIZE]
        for row in self.feature_weights.iter() {
            for &w in row.iter() {
                writer.write_all(&w.to_le_bytes())?;
            }
        }
        // Feature biases
        for &b in self.feature_biases.iter() {
            writer.write_all(&b.to_le_bytes())?;
        }
        // Hidden1 weights: [HIDDEN1_SIZE][HIDDEN2_SIZE]
        for row in self.hidden1_weights.iter() {
            for &w in row.iter() {
                writer.write_all(&w.to_le_bytes())?;
            }
        }
        // Hidden1 biases
        for &b in self.hidden1_biases.iter() {
            writer.write_all(&b.to_le_bytes())?;
        }
        // Hidden2 weights
        for &w in self.hidden2_weights.iter() {
            writer.write_all(&w.to_le_bytes())?;
        }
        // Hidden2 bias
        writer.write_all(&self.hidden2_bias.to_le_bytes())?;

        Ok(())
    }

    /// Load weights from .fnnue binary format.
    pub fn load<R: Read>(reader: &mut R) -> io::Result<Self> {
        // Read header
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if magic != FNNUE_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid .fnnue magic bytes",
            ));
        }

        let arch_hash = read_u32_le(reader)?;
        if arch_hash != Self::architecture_hash() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Architecture hash mismatch: file={arch_hash:#x}, expected={:#x}",
                    Self::architecture_hash()
                ),
            ));
        }

        let num_features = read_u32_le(reader)? as usize;
        let hidden1_size = read_u32_le(reader)? as usize;
        let hidden2_size = read_u32_le(reader)? as usize;
        let output_size = read_u32_le(reader)? as usize;

        if num_features != TOTAL_FEATURES
            || hidden1_size != HIDDEN1_SIZE
            || hidden2_size != HIDDEN2_SIZE
            || output_size != 1
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Layer size mismatch: file=({num_features},{hidden1_size},{hidden2_size},{output_size}), \
                     expected=({TOTAL_FEATURES},{HIDDEN1_SIZE},{HIDDEN2_SIZE},1)"
                ),
            ));
        }

        let mut weights = Self::zeros();

        // Feature weights
        for row in weights.feature_weights.iter_mut() {
            for w in row.iter_mut() {
                *w = read_i16_le(reader)?;
            }
        }
        // Feature biases
        for b in weights.feature_biases.iter_mut() {
            *b = read_i16_le(reader)?;
        }
        // Hidden1 weights
        for row in weights.hidden1_weights.iter_mut() {
            for w in row.iter_mut() {
                *w = read_i16_le(reader)?;
            }
        }
        // Hidden1 biases
        for b in weights.hidden1_biases.iter_mut() {
            *b = read_i16_le(reader)?;
        }
        // Hidden2 weights
        for w in weights.hidden2_weights.iter_mut() {
            *w = read_i16_le(reader)?;
        }
        // Hidden2 bias
        weights.hidden2_bias = read_i16_le(reader)?;

        Ok(weights)
    }

    /// Load weights from a file path.
    pub fn from_file(path: &str) -> io::Result<Self> {
        let mut file = std::fs::File::open(path)?;
        Self::load(&mut file)
    }

    /// Save weights to a file path.
    pub fn to_file(&self, path: &str) -> io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.save(&mut file)
    }
}

// ─── Binary I/O Helpers ────────────────────────────────────────────────────

fn read_u32_le<R: Read>(reader: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i16_le<R: Read>(reader: &mut R) -> io::Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

// ─── Xorshift64 PRNG ──────────────────────────────────────────────────────

/// Simple xorshift64 PRNG for deterministic random weight generation.
struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate a random i16 in [lo, hi] (inclusive).
    fn next_i16_range(&mut self, lo: i16, hi: i16) -> i16 {
        let range = (hi as i32 - lo as i32 + 1) as u32;
        let val = (self.next_u64() % range as u64) as i32 + lo as i32;
        val as i16
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_weights_deterministic() {
        let w1 = NnueWeights::random(42);
        let w2 = NnueWeights::random(42);
        assert_eq!(w1.feature_biases, w2.feature_biases);
        assert_eq!(w1.hidden1_biases, w2.hidden1_biases);
        assert_eq!(w1.hidden2_weights, w2.hidden2_weights);
        assert_eq!(w1.hidden2_bias, w2.hidden2_bias);
    }

    #[test]
    fn test_different_seeds_different_weights() {
        let w1 = NnueWeights::random(42);
        let w2 = NnueWeights::random(123);
        assert_ne!(w1.feature_biases, w2.feature_biases);
    }

    #[test]
    fn test_save_load_round_trip() {
        let original = NnueWeights::random(42);

        // Save to memory buffer
        let mut buf = Vec::new();
        original.save(&mut buf).unwrap();

        // Load back
        let mut cursor = io::Cursor::new(&buf);
        let loaded = NnueWeights::load(&mut cursor).unwrap();

        // Verify exact match
        assert_eq!(original.feature_biases, loaded.feature_biases);
        assert_eq!(original.hidden1_biases, loaded.hidden1_biases);
        assert_eq!(original.hidden2_weights, loaded.hidden2_weights);
        assert_eq!(original.hidden2_bias, loaded.hidden2_bias);

        // Spot-check feature weights
        for i in 0..10 {
            assert_eq!(original.feature_weights[i], loaded.feature_weights[i]);
        }
        // Check last few rows too
        for i in (TOTAL_FEATURES - 5)..TOTAL_FEATURES {
            assert_eq!(original.feature_weights[i], loaded.feature_weights[i]);
        }

        // Check hidden1 weights
        for i in 0..HIDDEN1_SIZE {
            assert_eq!(original.hidden1_weights[i], loaded.hidden1_weights[i]);
        }
    }

    #[test]
    fn test_invalid_magic_rejected() {
        let mut buf = vec![0u8; HEADER_SIZE + 100]; // garbage data
        buf[0..4].copy_from_slice(b"NOPE");
        let mut cursor = io::Cursor::new(&buf);
        let result = NnueWeights::load(&mut cursor);
        assert!(result.is_err());
        let err = result.as_ref().err().unwrap();
        assert!(err.to_string().contains("magic"));
    }

    #[test]
    fn test_architecture_hash_consistent() {
        let h1 = NnueWeights::architecture_hash();
        let h2 = NnueWeights::architecture_hash();
        assert_eq!(h1, h2);
        assert_ne!(h1, 0);
    }

    #[test]
    fn test_save_file_size() {
        let weights = NnueWeights::random(42);
        let mut buf = Vec::new();
        weights.save(&mut buf).unwrap();

        let expected_size = HEADER_SIZE
            + TOTAL_FEATURES * HIDDEN1_SIZE * 2 // feature_weights
            + HIDDEN1_SIZE * 2                   // feature_biases
            + HIDDEN1_SIZE * HIDDEN2_SIZE * 2    // hidden1_weights
            + HIDDEN2_SIZE * 2                   // hidden1_biases
            + HIDDEN2_SIZE * 2                   // hidden2_weights
            + 2; // hidden2_bias

        assert_eq!(buf.len(), expected_size);
    }

    #[test]
    fn test_zeros() {
        let w = NnueWeights::zeros();
        assert_eq!(w.hidden2_bias, 0);
        assert!(w.feature_biases.iter().all(|&v| v == 0));
    }
}
