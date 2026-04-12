use steadfast_bytes::{
    AsArraySelf, ByteSize, BytesErr, FromBytes, ToBytes, TryReadBytes, TryWriteBytes, TypeCode,
    TypeCoded,
};
/// See "Secure Hash Standard" in FIPS PUB 180-4 on [NIST](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SHA256([u32; 8]);

impl std::fmt::Display for SHA256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex_digest())
    }
}

impl Default for SHA256 {
    fn default() -> Self {
        Self(SHA256::H)
    }
}
#[derive(Debug)]
enum RemainderChunk {
    One([u8; 64]),
    Two([u8; 64], [u8; 64]),
}

impl SHA256 {
    pub fn inner_bytes(&self) -> &[u32; 8] {
        &self.0
    }
    pub const fn from_raw(inner_bytes: [u32; 8]) -> Self {
        Self(inner_bytes)
    }
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

    fn convert_byte_chunk(chunk: &[u8; 64]) -> [u32; 64] {
        chunk
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .fold([0; 64], |mut w, (i, chunk)| {
                w[i] = u32::from_be_bytes(*chunk);
                w
            })
    }

    fn expand_chunk(chunk: &[u8; 64]) -> [u32; 64] {
        (16..64).fold(Self::convert_byte_chunk(chunk), |mut w, i| {
            w[i] = w[i - 16]
                .wrapping_add(Self::lower_sigma(w[i - 15], 7, 18, 3))
                .wrapping_add(w[i - 7])
                .wrapping_add(Self::lower_sigma(w[i - 2], 17, 19, 10));
            w
        })
    }

    fn apply_sum(&mut self, expanded_chunk: [u32; 64], i: usize) {
        let t1 = self.0[7]
            .wrapping_add(self.upper_sigma(4, 6, 11, 25))
            .wrapping_add(self.ch())
            .wrapping_add(Self::K[i])
            .wrapping_add(expanded_chunk[i]);

        let t2 = self.upper_sigma(0, 2, 13, 22).wrapping_add(self.maj());

        self.0[7] = self.0[6];
        self.0[6] = self.0[5];
        self.0[5] = self.0[4];
        self.0[4] = self.0[3].wrapping_add(t1);
        self.0[3] = self.0[2];
        self.0[2] = self.0[1];
        self.0[1] = self.0[0];
        self.0[0] = t1.wrapping_add(t2);
    }

    fn apply_chunk(mut self, chunk: &[u8; 64]) -> Self {
        let expanded_chunk = Self::expand_chunk(chunk);

        let mut h = [0; 8];
        h.copy_from_slice(&self.0);

        for i in 0..64 {
            self.apply_sum(expanded_chunk, i);
        }

        (0..8).zip(h.into_iter()).fold(self, |mut hash, (i, b)| {
            hash.0[i] = b.wrapping_add(hash.0[i]);
            hash
        })
    }

    fn pad_chunk(chunk: &[u8], total_len: usize) -> RemainderChunk {
        let bit_len = (total_len as u64) * 8;

        let mut block_1 = [0u8; 64];
        block_1[..chunk.len()].copy_from_slice(chunk);
        block_1[chunk.len()] = 0x80;

        if chunk.len() <= 55 {
            block_1[56..64].copy_from_slice(&bit_len.to_be_bytes());
            RemainderChunk::One(block_1)
        } else {
            let mut block_2 = [0u8; 64];
            block_2[56..64].copy_from_slice(&bit_len.to_be_bytes());
            RemainderChunk::Two(block_1, block_2)
        }
    }

    fn chunk_data<'a>(data: &'a [u8]) -> (&'a [[u8; 64]], RemainderChunk) {
        let (chunks, remainder) = data.as_chunks::<64>();
        let data_len = data.len();
        (chunks, Self::pad_chunk(remainder, data_len))
    }

    pub fn combine(self, other: &Self) -> Self {
        let mut bytes = [0u8; 64];
        bytes
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(
                self.0
                    .iter()
                    .map(|num| num.to_le_bytes())
                    .chain(other.0.iter().map(|num| num.to_le_bytes())),
            )
            .for_each(|(chunk, num_chunk)| chunk.copy_from_slice(&num_chunk));

        self.apply_chunk(&bytes)
    }

    pub fn new(data: &[u8]) -> Self {
        let (chunks, remainder_chunks) = Self::chunk_data(data);
        let hash = chunks
            .iter()
            .fold(SHA256::default(), |hash, chunk| hash.apply_chunk(chunk));

        match remainder_chunks {
            RemainderChunk::One(chunk) => hash.apply_chunk(&chunk),
            RemainderChunk::Two(chunk_a, chunk_b) => {
                hash.apply_chunk(&chunk_a).apply_chunk(&chunk_b)
            }
        }
    }

    pub fn hex_digest(&self) -> String {
        format!(
            "{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5], self.0[6], self.0[7],
        )
    }
}

impl ByteSize for SHA256 {
    const BYTE_SIZE: usize = 32;
}

impl TypeCoded for SHA256 {
    const TYPE_CODE: TypeCode = TypeCode::Extension(19);
}

macro_rules! impl_trb_sha256 {
    ($fn_name:ident) => {
        fn $fn_name<R: std::io::Read>(
            stream: &mut R,
            checksum: &mut usize,
        ) -> Result<Self, BytesErr> {
            let mut buf = [0u32; 8];
            for i in buf.iter_mut() {
                *i = <u32>::$fn_name(stream, checksum)?;
            }
            Ok(Self::from_raw(buf))
        }
    };
}

impl TryReadBytes for SHA256 {
    impl_trb_sha256!(try_read_bytes_le);
    impl_trb_sha256!(try_read_bytes_be);
    impl_trb_sha256!(try_read_bytes_ne);
}

macro_rules! impl_twb_sha256 {
    ($fn_name: ident) => {
        fn $fn_name<W: std::io::Write>(&self, stream: &mut W) -> Result<usize, BytesErr> {
            let mut byte_count = 0;
            for num in self.0 {
                byte_count += num.$fn_name(stream)?;
            }

            Ok(byte_count)
        }
    };
}

impl TryWriteBytes for SHA256 {
    impl_twb_sha256!(try_write_bytes_le);
    impl_twb_sha256!(try_write_bytes_be);
    impl_twb_sha256!(try_write_bytes_ne);
}

macro_rules! impl_fb_sha256 {
    ($fn_name: ident, $fb: ident) => {
        fn $fn_name(bytes: T) -> Self {
            Self::from_raw(
                bytes
                    .as_array_self()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .enumerate()
                    .fold([0; 8], |mut num, (i, chunk)| {
                        num[i] = <u32>::$fb(*chunk);
                        num
                    }),
            )
        }
    };
}

impl<T> FromBytes<T> for SHA256
where
    T: AsArraySelf<32>,
{
    impl_fb_sha256!(from_bytes_le, from_le_bytes);
    impl_fb_sha256!(from_bytes_be, from_be_bytes);
    impl_fb_sha256!(from_bytes_ne, from_ne_bytes);
}

macro_rules! impl_tb_sha256 {
    ($fn_name: ident, $tb: ident) => {
        fn $fn_name(&self) -> [u8; 32] {
            let mut bytes = [0u8; 32];
            bytes
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .zip(self.inner_bytes().iter().map(|num| num.$tb()))
                .for_each(|(chunk, num_chunk)| chunk.copy_from_slice(&num_chunk));
            bytes
        }
    };
}

impl ToBytes<[u8; 32]> for SHA256 {
    impl_tb_sha256!(to_bytes_le, to_le_bytes);
    impl_tb_sha256!(to_bytes_be, to_be_bytes);
    impl_tb_sha256!(to_bytes_ne, to_ne_bytes);
}
#[cfg(test)]
mod tests {
    use super::*;

    //code used to generate tests:
    // import hashlib
    // import random
    // import string

    // def generate_random_string(length):
    //     characters = string.ascii_letters + string.digits
    //     return ''.join(random.choices(characters, k=length))

    // for i in range(0,1000):
    //     text = generate_random_string(i)
    //     hash_object = hashlib.sha256(text.encode())
    //     hex_dig = hash_object.hexdigest()
    //     print(f"assert_eq!(\n\tSHA256::new(b\"{text}\").hex_digest(),\n\tString::from(\"{hex_dig}\")\n);")

    #[test]
    fn it_works() {
        assert_eq!(
            SHA256::new(b"").hex_digest(),
            String::from("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert_eq!(
            SHA256::new(b"4").hex_digest(),
            String::from("4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a")
        );
        assert_eq!(
            SHA256::new(b"iX").hex_digest(),
            String::from("8a2aeda2d8fd5cd26ce72400df8141b0f56ae47b4073ca8b2864b51535cc6347")
        );
        assert_eq!(
            SHA256::new(b"o6n").hex_digest(),
            String::from("a58785e671d7c363d94c718302df8f3bd6325ae657eb60b8c1f537f2850aafa4")
        );
        assert_eq!(
            SHA256::new(b"xn5Q").hex_digest(),
            String::from("be6737164bccf521660ba7777d3871cd64405c4279a2469e2c9c734174986ef4")
        );
        assert_eq!(
            SHA256::new(b"nwW1q").hex_digest(),
            String::from("16550e202dd6084f2e8bc021a25f20bf5f277cf997c4031e39729b85f09de304")
        );
        assert_eq!(
            SHA256::new(b"ksqtKM").hex_digest(),
            String::from("b63090107e4f2acc9e3d794ee28f0b5ed37acc18a13d2062b1e393b5e7ef7152")
        );
        assert_eq!(
        	SHA256::new(b"9qdBjVJjtnh1EtHa2cNGVxc4JBdm7ed6MzwGknwmXs9fwv8sE3iGd2FkcUZgX9dXFmF5a7if9ceYUCf7G7n0TnsIyZthzldLCqagwezUkjM46QPVXuQGkYLyXC7jOo").hex_digest(),
        	String::from("d74e5ba452404f0e10449da657258e88f8bd068281000380bb713e44253a2427")
        );
        assert_eq!(
        	SHA256::new(b"MPF8yualKiAnW8H0Yf7ozzwSrNngM8nxRyX2Qo6TFBEs39HI0CNEBHoFeUFTAOjZlpQRzFdMFWJgVM7R3dQJMQSEDS8Lw8q5eDUwyHCdKGNmLaqhO06ovljLclHZtJn").hex_digest(),
        	String::from("69a6cc9afb3689131d245959e9161c0fcdece04b43db6bc4bda5de2f336414a8")
        );
        assert_eq!(
        	SHA256::new(b"eHUXmyciHhQ5BbISTpSoZWyFckhSpKlJTEWADn84eMTr7E8ssjmFxCSU2xkXAZSwr3ZKqfxnJCyvhWZP343QmSWQBVXeRLI0DaRSjVPLi99M1hMhCMTdYZoiL2Sk3Ily").hex_digest(),
        	String::from("0ea97fb7d25faa21a13929e5d5603db0e9c20ea47ff8ea3c225e4cb61672b463")
        );
        assert_eq!(
        	SHA256::new(b"Ixcf07J57gC9pXBXDJXwjUYY3QOvx3YuwnnDcvr17ZLhZW1mPwiZFgfhKeBRDnb733qSh4Cpy4RaaOqKtPTGhz0Au4UAOjuKqULzQwxp6xutrEoYmKVf6ZKfqIxjXEzuo").hex_digest(),
        	String::from("79f1a2310f44abfdfe7513d5192284d0ed89b602ca133b414bd92a9028aab22e")
        );
        assert_eq!(
        	SHA256::new(b"oYJZ1F0FN4SfmCb2HD9jaNGhq2tXfXTtE76fS59GvDK1uawWKzqzuXM51OgQoT8kejr0nLrNk0bhyFmH7zvJR3Ev73nWxwa8iqlNgO1XqK3xyvHblw3g2Xh48jL3QS0blVByt8IdL79DoGm2aPOwHboJpxuIqr9hMbxZvr3wYx").hex_digest(),
        	String::from("2dbddf548c8fd6ba69c7fe64ac5496f4f2bf2a6bea81d7477d274d7a63604939")
        );
        assert_eq!(
        	SHA256::new(b"gPnkQ47DonIUG9SpPjxRIPhncXYBVer5zO736Z7JUbUNVI5Na3tt75MmoOZ9TfwP8vK8Pt9wzGz9GxmIpgreNH40gIa1SbN5xVIKnhWMbBiEKdV9gJ8SMMhrramc6oTHDrsEzHnkMpmxUKBp1qqmGJdUHmqpAXJeU6RtcJwfY03MAmDFTW3LzMG1ZIsXvjIuXnVUJKXhVUjqq7BM07qpkAVCp6Wp2NsOC9SBgtJ05iBNLmCp1IKHCgTO4Qsa7xJHr4YqCZSKP3Jh2teRLBDADVXG3PIMpbx0xPaLABK48fpKdVNZwlbOAsAjRP1Rso8rDHZq6Vwya0GFql2UisZ9nRe9vZKkNrGWayWbRiHf8CYUqoFJdGw6XAJYDZ2tZWpUpIQ90gkYJkXfsW8fPAE4oDjBKQQDmKiwTW1DlzgezMFhpBpmwaNqthVmrZZGgkOTCOi58Rs6KhpGzJSqRANnPQ2zObrFPIJSnBkuvS1Dvk0DuM9Cu2ZiJ8OSYH1Z0D2K5hYP3vBnH4lzTU49LRtosCosfOtHxs9ljT9YUotlYkx0heVQxsOfDBzF1lqwSUY7cmjxn4RkesVB8xeM2c6mcAu3Kw3M5jsqBqLocZkF20eZ3tUi0szlpVtmJ9ckiWeGBF031pJDnIkMQdoaF7icTnmW2soqMrvtIBoC6j0nRXPqeDOLl7Usm0cKCZO5XApXMUmMdW96MV2YtBtCDTwiQNGIZejVu7M8noqmvHYHeAFPXIN3dPGau12XGpdCI1u1kqewwfKbHD0bHzO9HMvIpnmKsQ1nc1COidlRZk5tG792u0hjoReDJ22m18UIGv5Y2kApFOci0tdYrNlP3LwiM1IaxElUST0fGxcQkNeJUVHDc4Iyo4RWjnLvlYPC3epsdNkiCu5y0HS5xvHQfvlASdSkyCQokcdXHqVRTGNx0JxiTqiVsUWk67wLBedkRk4eJqMZBH4XRK8Oq99C15zJFGxHl9cY5N23Qdvnqio").hex_digest(),
        	String::from("ef16b2199239b2ddedda5d158ada01405527de9c3aa7dcc3b87cc8e65f5d1796")
        );
    }
}
