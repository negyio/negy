use anyhow::Result;
use bytes::BytesMut;
use openssl::rand::rand_bytes;

pub struct EncryptedPayload {
    inner: BytesMut,
    skip_offset: usize,
}

pub const DELIMITER_LEN: usize = 16;

impl EncryptedPayload {
    pub fn new() -> Self {
        EncryptedPayload {
            inner: BytesMut::new(),
            skip_offset: 0,
        }
    }

    pub fn new_delimiter() -> [u8; DELIMITER_LEN] {
        let mut delimiter = [0; DELIMITER_LEN];
        rand_bytes(&mut delimiter).unwrap();
        delimiter
    }

    pub fn read(
        &mut self,
        payload: &[u8],
        delimiter: &[u8; DELIMITER_LEN],
    ) -> Result<Vec<BytesMut>> {
        self.inner.extend_from_slice(payload);
        self.parse(delimiter)
    }

    fn parse(&mut self, delimiter: &[u8; DELIMITER_LEN]) -> Result<Vec<BytesMut>> {
        let mut payloads: Vec<BytesMut> = Vec::new();

        let delimiter_indices: Vec<usize> = self
            .inner
            .windows(DELIMITER_LEN)
            .enumerate()
            .skip_while(|(idx, _)| *idx < self.skip_offset)
            .filter_map(
                |(idx, bytes)| {
                    if bytes == delimiter {
                        Some(idx)
                    } else {
                        None
                    }
                },
            )
            .collect();

        let mut offset = 0;

        for &idx in delimiter_indices.iter() {
            payloads.push(self.inner.split_to(idx - offset));
            let _ = self.inner.split_to(DELIMITER_LEN);
            offset = idx + DELIMITER_LEN;
        }

        self.skip_offset = self.inner.len();

        Ok(payloads)
    }
}
