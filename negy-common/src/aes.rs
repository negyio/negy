use anyhow::Result;
use openssl::rand::rand_bytes;
use openssl::symm::{decrypt, encrypt, Cipher};

#[derive(Debug)]
pub struct Aes {
    key: [u8; 32],
    iv: [u8; 16],
}

impl Aes {
    pub fn new() -> Self {
        let mut key = [0; 32];
        let mut iv = [0; 16];

        rand_bytes(&mut key).unwrap();
        rand_bytes(&mut iv).unwrap();

        Aes { key, iv }
    }

    pub fn import(key_iv: &[u8; 48]) -> Self {
        let mut key = [0; 32];
        let mut iv = [0; 16];

        key.copy_from_slice(&key_iv[..32]);
        iv.copy_from_slice(&key_iv[32..]);

        Aes { key, iv }
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(encrypt(
            Cipher::aes_256_cbc(),
            &self.key,
            Some(&self.iv),
            data,
        )?)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(decrypt(
            Cipher::aes_256_cbc(),
            &self.key,
            Some(&self.iv),
            data,
        )?)
    }

    pub fn get_key_iv(&self) -> [u8; 48] {
        let mut key_iv = [0; 48];

        key_iv[..32].copy_from_slice(&self.key);
        key_iv[32..].copy_from_slice(&self.iv);

        key_iv
    }
}
