/// See "Secure Hash Standard" in FIPS PUB 180-4 on [NIST](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SHA256([u32; 8]);

impl SHA256 {
    pub const W: u32 = 32;
    pub const M: u64 = 0x100000000;
    pub const FF: u32 = (Self::M - 1) as u32;

    /// See NIST FIPS 180-4 Section 4.2.2
    ///
    /// SHA-224 and SHA-256 use the same sequence of sixty-four constant 32-bit
    /// words, K[0], K[1], ..., K[63]. These words represent the first thirty-two bits
    /// of the fractional parts of the cube roots of the first sixty-four prime numbers.
    pub const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    /// See NIST FIPS 180-4 Section 5.3.3
    ///
    /// For SHA-256, the initial hash value, H, shall consist of the
    /// following eight 32-bit words, in hex: (See const array below).
    /// These words were obtained by taking the first thirty-two bits of the
    /// fractional parts of the square roots of the first eight prime numbers
    pub const H: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    /// See NIST FIPS 180-4 Section 4.1.2 (4.4-4.5)
    fn upper_sigma(&self, idx: usize, offset_a: u32, offset_b: u32, offset_c: u32) -> u32 {
        self.0[idx].rotate_right(offset_a)
            ^ self.0[idx].rotate_right(offset_b)
            ^ self.0[idx].rotate_right(offset_c)
    }

    /// See NIST FIPS 180-4 Section 4.1.2 (4.6-4.7)
    fn lower_sigma(x: u32, offset_a: u32, offset_b: u32, offset_c: u32) -> u32 {
        x.rotate_right(offset_a) ^ x.rotate_right(offset_b) ^ (x >> offset_c)
    }

    /// See NIST FIPS 180-4 Section 4.1.2 (4.2)
    fn ch(&self) -> u32 {
        (self.0[4] & self.0[5]) ^ (!self.0[4] & self.0[6])
    }

    /// See NIST FIPS 180-4 Section 4.1.2 (4.3)
    fn maj(&self) -> u32 {
        (self.0[0] & self.0[1]) ^ (self.0[0] & self.0[2]) ^ (self.0[1] & self.0[2])
    }

    fn apply_segment(mut self, segment: &[u8]) -> Self {
        let mut w = segment
            .chunks_exact(4)
            .enumerate()
            .fold([0; 64], |mut w, (i, chunk)| {
                w[i] = chunk
                    .first_chunk::<4>()
                    .map(|chunk| u32::from_be_bytes(*chunk))
                    .expect("Exact chunks of 4 somehow failed.");
                w
            });

        (16..64).fold(&mut w, |w, i| {
            w[i] = w[i - 16]
                .wrapping_add(Self::lower_sigma(w[i - 15], 7, 18, 3))
                .wrapping_add(w[i - 7])
                .wrapping_add(Self::lower_sigma(w[i - 2], 17, 19, 10));
            w
        });

        let mut h = self.0;
        for i in 0..64 {
            let t0 = h[7]
                .wrapping_add(self.upper_sigma(0, 2, 13, 22))
                .wrapping_add(self.ch())
                .wrapping_add(Self::K[i])
                .wrapping_add(w[i]);

            let t1 = self.upper_sigma(4, 6, 11, 26).wrapping_add(self.maj());

            h[7] = h[6];
            h[6] = h[5];
            h[5] = h[4];
            h[4] = h[3].wrapping_add(t0);
            h[3] = h[2];
            h[2] = h[1];
            h[1] = h[0];
            h[0] = t0.wrapping_add(t1);
        }

        for (i, v) in self.0.iter_mut().enumerate() {
            *v = v.wrapping_add(h[i]);
        }

        self
    }

    fn pad_data(data: &[u8]) -> [u8; 64] {
        let mut padded_data = [0; 64];
        let data_len = data.len();
        padded_data[0..data_len].copy_from_slice(data);
        padded_data[data_len] = 0x80;
        (data_len as u16).to_be_bytes().iter().enumerate().fold(
            padded_data,
            |mut padded_data, (i, b)| {
                padded_data[data_len - 3 + i] |= b;
                padded_data
            },
        )
    }

    pub fn new(data: &[u8]) -> Self {
        let mut hash = if data.len() == 0 {
            let mut blank_data = [0; 64];
            blank_data[0] = 1;
            Self(Self::H).apply_segment(&blank_data)
        } else {
            data.chunks(64).fold(Self(Self::H), |hash, chunk| {
                if chunk.len() < 64 {
                    hash.apply_segment(&Self::pad_data(chunk))
                } else {
                    hash.apply_segment(chunk)
                }
            })
        };
        for h in hash.0.iter_mut() {
            *h = h.to_be();
        }
        hash
    }

    pub fn hex_digest(&self) -> String {
        format!(
            "{:x}{:x}{:x}{:x}{:x}{:x}{:x}{:x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let hash = SHA256::new(b"");

        // Doing something wrong somehow :(
        assert_eq!(
            hash.hex_digest(),
            String::from("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        )
    }
}
