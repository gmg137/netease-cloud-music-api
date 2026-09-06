//
// encrypt.rs
// Copyright (C) 2019 gmg137 <gmg137@live.com>
// Distributed under terms of the GPLv3 license.
//
use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use flate2::read::GzDecoder;
use lazy_static::lazy_static;
use openssl::derive::Deriver;
use openssl::hash::{hash, DigestBytes, MessageDigest};
use openssl::pkey::{Id, PKey};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::Signer;
use openssl::symm::{decrypt, encrypt, encrypt_aead, Cipher};
use regex::Regex;
use std::collections::HashMap;
use std::io::Read;
use urlqstring::QueryParams;
use AesMode::{cbc, ecb};

lazy_static! {
    static ref IV: Vec<u8> = "0102030405060708".as_bytes().to_vec();
    static ref PRESET_KEY: Vec<u8> = "0CoJUm6Qyw8W8jud".as_bytes().to_vec();
    static ref LINUX_API_KEY: Vec<u8> = "rFgB&h#%2?^eDg:Q".as_bytes().to_vec();
    static ref BASE62: Vec<u8> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".as_bytes().to_vec();
    static ref RSA_PUBLIC_KEY: Vec<u8> = "-----BEGIN PUBLIC KEY-----\nMIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDgtQn2JZ34ZC28NWYpAUd98iZ37BUrX/aKzmFbt7clFSs6sXqHauqKWqdtLkF2KexO40H1YTX8z2lSgBBOAxLsvaklV8k4cBFK9snQXE9/DDaFt6Rr7iVZMldczhC0JNgTz+SHXT6CBHuX3e9SdB1Ua44oncaTWz7OBGLbCiK45wIDAQAB\n-----END PUBLIC KEY-----".as_bytes().to_vec();
    static ref EAPIKEY: Vec<u8> = "e82ckenh8dichen8".as_bytes().to_vec();
    static ref XEAPI_STATIC_KEY: Vec<u8> = hex::decode("ab1d5a430f6bb04a3f01e81ddd72bd916d5ce591248ac128714806d7f8fb1b84").unwrap();
    static ref XEAPI_SIGN_KEY: Vec<u8> = "mUHCwVNWJbunMqAHf5MImuirT6plvs6VSFW62MGHstFQxhBGdEoIhLItH3djc4+FB/OKty3+lL2rGeoFBpVe5g==".as_bytes().to_vec();
}

#[allow(non_snake_case)]
pub struct Crypto;

#[derive(Clone)]
pub struct XeapiPublicKeyState {
    pub version: String,
    pub public_key: String,
    pub sk: String,
}

#[allow(dead_code, non_camel_case_types)]
pub enum HashType {
    md5,
}

#[allow(non_camel_case_types)]
pub enum AesMode {
    cbc,
    ecb,
}

#[allow(dead_code, clippy::redundant_closure)]
impl Crypto {
    pub fn hex_random_bytes(n: usize) -> String {
        let mut data: Vec<u8> = Vec::with_capacity(n);
        rand::fill(&mut data[..]);
        hex::encode(data)
    }

    pub fn eapi(url: &str, text: &str) -> String {
        let message = format!("nobody{}use{}md5forencrypt", url, text);
        let digest = hex::encode(hash(MessageDigest::md5(), message.as_bytes()).unwrap());
        let data = format!("{}-36cd479b6b5-{}-36cd479b6b5-{}", url, text, digest);
        let params = Crypto::aes_encrypt(&data, &EAPIKEY, ecb, None, |t: &Vec<u8>| {
            hex::encode_upper(t)
        });
        QueryParams::from(vec![("params", params.as_str())]).stringify()
    }

    pub fn aes_decrypt(
        data: &[u8],
        key: &[u8],
        mode: AesMode,
        iv: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        let cipher = match mode {
            cbc => Cipher::aes_128_cbc(),
            ecb => Cipher::aes_128_ecb(),
        };
        decrypt(cipher, key, iv, data).map_err(|_| anyhow!("aes decrypt failed"))
    }

    pub fn eapi_res_decrypt(encrypted_params: &str, aeapi: bool) -> Result<String> {
        let encrypted_bytes =
            hex::decode(encrypted_params).map_err(|_| anyhow!("hex decode failed"))?;
        let mut decrypted = Crypto::aes_decrypt(&encrypted_bytes, &EAPIKEY, ecb, None)?;

        let pad_len = decrypted[decrypted.len() - 1] as usize;
        decrypted.truncate(decrypted.len() - pad_len);

        if aeapi {
            let decoded = general_purpose::STANDARD
                .decode(&decrypted)
                .map_err(|_| anyhow!("base64 decode failed"))?;
            let mut decoder = GzDecoder::new(&decoded[..]);
            let mut result = String::new();
            decoder
                .read_to_string(&mut result)
                .map_err(|_| anyhow!("gzip decompress failed"))?;
            Ok(result)
        } else {
            String::from_utf8(decrypted).map_err(|_| anyhow!("utf8 decode failed"))
        }
    }

    pub fn eapi_req_decrypt(encrypted_params: &str) -> Result<(String, String)> {
        let encrypted_bytes =
            hex::decode(encrypted_params).map_err(|_| anyhow!("hex decode failed"))?;
        let mut decrypted = Crypto::aes_decrypt(&encrypted_bytes, &EAPIKEY, ecb, None)?;

        let pad_len = decrypted[decrypted.len() - 1] as usize;
        decrypted.truncate(decrypted.len() - pad_len);

        let text =
            String::from_utf8(decrypted).map_err(|_| anyhow!("utf8 decode failed"))?;

        let re =
            Regex::new(r"(.*?)-36cd479b6b5-(.*?)-36cd479b6b5-(.*)").map_err(|_| anyhow!("regex error"))?;
        if let Some(caps) = re.captures(&text) {
            let url = caps.get(1).unwrap().as_str().to_string();
            let data = caps.get(2).unwrap().as_str().to_string();
            Ok((url, data))
        } else {
            Err(anyhow!("eapi_req_decrypt: invalid format"))
        }
    }

    pub fn weapi(text: &str) -> String {
        let mut secret_key = [0u8; 16];
        rand::fill(&mut secret_key[..]);
        let key: Vec<u8> = secret_key
            .iter()
            .map(|i| BASE62[(i % 62) as usize])
            .collect();

        let params1 = Crypto::aes_encrypt(text, &PRESET_KEY, cbc, Some(&*IV), |t: &Vec<u8>| {
            general_purpose::STANDARD.encode(t)
        });

        let params = Crypto::aes_encrypt(&params1, &key, cbc, Some(&*IV), |t: &Vec<u8>| {
            general_purpose::STANDARD.encode(t)
        });

        let enc_sec_key = Crypto::rsa_encrypt(
            std::str::from_utf8(&key.iter().rev().copied().collect::<Vec<u8>>()).unwrap(),
            &RSA_PUBLIC_KEY,
        );

        QueryParams::from(vec![
            ("params", params.as_str()),
            ("encSecKey", enc_sec_key.as_str()),
        ])
        .stringify()
    }

    pub fn linuxapi(text: &str) -> String {
        let params = Crypto::aes_encrypt(text, &LINUX_API_KEY, ecb, None, |t: &Vec<u8>| {
            hex::encode(t)
        })
        .to_uppercase();
        QueryParams::from(vec![("eparams", params.as_str())]).stringify()
    }

    pub fn aes_encrypt(
        data: &str,
        key: &[u8],
        mode: AesMode,
        iv: Option<&[u8]>,
        encode: fn(&Vec<u8>) -> String,
    ) -> String {
        let cipher = match mode {
            cbc => Cipher::aes_128_cbc(),
            ecb => Cipher::aes_128_ecb(),
        };
        let cipher_text = encrypt(cipher, key, iv, data.as_bytes()).unwrap();

        encode(&cipher_text)
    }

    pub fn rsa_encrypt(data: &str, key: &[u8]) -> String {
        let rsa = Rsa::public_key_from_pem(key).unwrap();

        let prefix = vec![0u8; 128 - data.len()];

        let data = [&prefix[..], data.as_bytes()].concat();

        let mut buf = vec![0; rsa.size() as usize];

        rsa.public_encrypt(&data, &mut buf, Padding::NONE).unwrap();

        hex::encode(buf)
    }

    #[allow(dead_code)]
    pub fn hash_encrypt(
        data: &str,
        algorithm: HashType,
        encode: fn(DigestBytes) -> String,
    ) -> String {
        match algorithm {
            HashType::md5 => encode(hash(MessageDigest::md5(), data.as_bytes()).unwrap()),
        }
    }

    pub fn aes_ecb_encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        let cipher = match key.len() {
            16 => Cipher::aes_128_ecb(),
            24 => Cipher::aes_192_ecb(),
            32 => Cipher::aes_256_ecb(),
            _ => return Err(anyhow!("invalid aes key length")),
        };
        encrypt(cipher, key, None, data).map_err(|_| anyhow!("aes ecb encrypt failed"))
    }

    pub fn aes_ecb_decrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        let cipher = match key.len() {
            16 => Cipher::aes_128_ecb(),
            24 => Cipher::aes_192_ecb(),
            32 => Cipher::aes_256_ecb(),
            _ => return Err(anyhow!("invalid aes key length")),
        };
        decrypt(cipher, key, None, data).map_err(|_| anyhow!("aes ecb decrypt failed"))
    }

    pub fn xeapi_sign(timestamp: &str, nonce: &str) -> String {
        let mut data = Vec::with_capacity(timestamp.len() + nonce.len());
        data.extend_from_slice(timestamp.as_bytes());
        data.extend_from_slice(nonce.as_bytes());
        general_purpose::STANDARD.encode(hmac_sha256(&XEAPI_SIGN_KEY, &data).unwrap())
    }

    pub fn xeapi_decrypt_public_key(encrypted_data: &str) -> Result<XeapiPublicKeyState> {
        let bytes = general_purpose::STANDARD
            .decode(encrypted_data)
            .map_err(|_| anyhow!("base64 decode failed"))?;
        let decrypted = Crypto::aes_ecb_decrypt(&XEAPI_STATIC_KEY, &bytes)?;
        let json: serde_json::Value =
            serde_json::from_str(&String::from_utf8(decrypted).map_err(|_| anyhow!("utf8 failed"))?)
                .map_err(|_| anyhow!("json parse failed"))?;
        let version = match json.get("version") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        Ok(XeapiPublicKeyState {
            version,
            public_key: json
                .get("publicKey")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            sk: json
                .get("sk")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    pub fn xeapi_mid_transform(ciphertext: &[u8]) -> Vec<u8> {
        let mut random = [0u8; 16];
        rand::fill(&mut random[..]);
        let xored: Vec<u8> = ciphertext
            .iter()
            .enumerate()
            .map(|(i, &b)| b ^ random[i & 0x0f])
            .collect();
        let b64 = general_purpose::STANDARD.encode(&xored);
        let b64_bytes = b64.as_bytes();
        let b64len = b64_bytes.len();
        let rot = (random[0] & 0x0f) as usize % b64len;
        let mut result = Vec::with_capacity(16 + b64len);
        result.extend_from_slice(&random);
        result.extend_from_slice(&b64_bytes[rot..]);
        result.extend_from_slice(&b64_bytes[..rot]);
        result
    }

    pub fn xeapi_encrypt_s(
        dynamic_key: &[u8],
        public_key_state: &XeapiPublicKeyState,
        os: &str,
    ) -> Result<Vec<u8>> {
        let peer_raw = general_purpose::STANDARD
            .decode(&public_key_state.public_key)
            .map_err(|_| anyhow!("base64 decode failed"))?;
        let peer = PKey::public_key_from_raw_bytes(&peer_raw, Id::X25519)
            .map_err(|_| anyhow!("x25519 public key import failed"))?;
        let ephemeral = PKey::generate_x25519()
            .map_err(|_| anyhow!("x25519 keypair generation failed"))?;
        let ephemeral_raw = ephemeral
            .raw_public_key()
            .map_err(|_| anyhow!("raw public key extraction failed"))?;
        let mut deriver = Deriver::new(&ephemeral).map_err(|_| anyhow!("deriver init failed"))?;
        deriver
            .set_peer(&peer)
            .map_err(|_| anyhow!("set peer failed"))?;
        let shared_secret = deriver
            .derive_to_vec()
            .map_err(|_| anyhow!("ecdh derive failed"))?;

        let zeros32 = vec![0u8; 32];
        let ss = if shared_secret.is_empty() {
            &zeros32[..]
        } else {
            &shared_secret[..]
        };
        let prk = hmac_sha256(&zeros32, ss)?;
        let mut info = ephemeral_raw.clone();
        info.push(1u8);
        let okm = hmac_sha256(&prk, &info)?;
        let aes_key = &okm[..16];

        let mut iv = [0u8; 12];
        rand::fill(&mut iv[..]);
        let plaintext = format!(
            "{}|{}|{}",
            general_purpose::STANDARD.encode(dynamic_key),
            os,
            public_key_state.sk
        );
        let cipher = Cipher::aes_128_gcm();
        let mut tag = [0u8; 16];
        let ciphertext = encrypt_aead(cipher, aes_key, Some(&iv), &[], plaintext.as_bytes(), &mut tag)
            .map_err(|_| anyhow!("aes gcm encrypt failed"))?;

        let mut result =
            Vec::with_capacity(ephemeral_raw.len() + iv.len() + ciphertext.len() + tag.len());
        result.extend_from_slice(&ephemeral_raw);
        result.extend_from_slice(&iv);
        result.extend_from_slice(&ciphertext);
        result.extend_from_slice(&tag);
        Ok(result)
    }

    pub fn build_xeapi_plaintext(params: &HashMap<&str, &str>) -> String {
        let pairs: Vec<(&str, &str)> = params
            .iter()
            .filter(|(k, _)| **k != "e_r")
            .map(|(k, v)| (*k, *v))
            .collect();
        let body_string = QueryParams::from(pairs).stringify();
        let body_b64 = general_purpose::STANDARD.encode(body_string.as_bytes());
        serde_json::json!({
            "body": body_b64,
            "queryString": "e_r=true",
        })
        .to_string()
    }

    pub fn xeapi(
        _path: &str,
        params: &HashMap<&str, &str>,
        public_key_state: &XeapiPublicKeyState,
    ) -> Result<String> {
        let mut dynamic_key = [0u8; 16];
        rand::fill(&mut dynamic_key[..]);
        let plaintext = Crypto::build_xeapi_plaintext(params);
        let inner = Crypto::aes_ecb_encrypt(&XEAPI_STATIC_KEY, plaintext.as_bytes())?;
        let mid = Crypto::xeapi_mid_transform(&inner);
        let b = Crypto::aes_ecb_encrypt(&dynamic_key, &mid)?;
        let s = Crypto::xeapi_encrypt_s(&dynamic_key, public_key_state, "android")?;
        let r = Crypto::aes_ecb_encrypt(
            &XEAPI_STATIC_KEY,
            format!("{}|", public_key_state.version).as_bytes(),
        )?;
        let b_str = general_purpose::STANDARD.encode(&b);
        let s_str = general_purpose::STANDARD.encode(&s);
        let r_str = general_purpose::STANDARD.encode(&r);
        Ok(QueryParams::from(vec![
            ("B", b_str.as_str()),
            ("S", s_str.as_str()),
            ("R", r_str.as_str()),
        ])
        .stringify())
    }

    pub fn xeapi_res_decrypt(body: &[u8]) -> Result<String> {
        let decrypted = Crypto::aes_ecb_decrypt(&EAPIKEY, body)?;
        if decrypted.len() >= 2 && decrypted[0] == 0x1f && decrypted[1] == 0x8b {
            let mut decoder = GzDecoder::new(&decrypted[..]);
            let mut result = String::new();
            decoder
                .read_to_string(&mut result)
                .map_err(|_| anyhow!("gzip decompress failed"))?;
            Ok(result)
        } else {
            String::from_utf8(decrypted).map_err(|_| anyhow!("utf8 decode failed"))
        }
    }

}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let pkey = PKey::hmac(key).map_err(|_| anyhow!("hmac key init failed"))?;
    let mut signer = Signer::new(MessageDigest::sha256(), &pkey)
        .map_err(|_| anyhow!("signer init failed"))?;
    signer.update(data).map_err(|_| anyhow!("signer update failed"))?;
    signer
        .sign_to_vec()
        .map_err(|_| anyhow!("sign failed"))
}
