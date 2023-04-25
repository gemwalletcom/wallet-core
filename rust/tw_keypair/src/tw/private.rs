// Copyright © 2017-2023 Trust Wallet.
//
// This file is part of Trust. The full Trust copyright notice, including
// terms governing use, modification, and redistribution, is contained in the
// file LICENSE at the root of the source code distribution tree.

use crate::traits::SigningKeyTrait;
use crate::tw::{TWCurve, TWPublicKey, TWPublicKeyType};
use crate::{secp256k1, starkex, Error};
use std::ops::Range;
use tw_hash::H256;
use tw_memory::ffi::RawPtrTrait;
use tw_utils::traits::ToBytesVec;
use zeroize::ZeroizeOnDrop;

/// Represents a private key that can be used to sign messages with different elliptic curves.
///
/// TODO add `secp256k1: Once<each_curve::PrivateKey>` for each curve.
#[derive(ZeroizeOnDrop)]
pub struct TWPrivateKey {
    bytes: Vec<u8>,
}

impl RawPtrTrait for TWPrivateKey {}

impl TWPrivateKey {
    /// The number of bytes in a private key.
    const SIZE: usize = 32;

    const KEY_RANGE: Range<usize> = 0..Self::SIZE;

    /// Validates the given `bytes` secret and creates a private key.
    pub fn new(bytes: Vec<u8>) -> Result<TWPrivateKey, Error> {
        if !Self::is_valid_general(&bytes) {
            return Err(Error::InvalidSecretKey);
        }
        Ok(TWPrivateKey { bytes })
    }

    /// Returns the 32 byte array - the essential private key data.
    pub fn key(&self) -> H256 {
        assert!(
            self.bytes.len() >= Self::SIZE,
            "'TWPrivateKey::bytes' has an unexpected length"
        );
        H256::try_from(&self.bytes[Self::KEY_RANGE])
            .expect("H256 and KEY_RANGE must be 32 byte length")
    }

    /// Checks if the given `bytes` secret is valid in general (without a concrete curve).
    pub fn is_valid_general(bytes: &[u8]) -> bool {
        if bytes.len() != Self::SIZE {
            return false;
        }
        // Check for zero address.
        !bytes.iter().all(|byte| *byte == 0)
    }

    /// Checks if the given `bytes` secret is valid.
    pub fn is_valid(bytes: &[u8], curve: TWCurve) -> bool {
        if !Self::is_valid_general(bytes) {
            return false;
        }
        match curve {
            TWCurve::Secp256k1 => secp256k1::PrivateKey::try_from(&bytes[Self::KEY_RANGE]).is_ok(),
            TWCurve::Starkex => starkex::PrivateKey::try_from(&bytes[Self::KEY_RANGE]).is_ok(),
        }
    }

    /// Signs a `hash` with using the given elliptic curve.
    pub fn sign(&self, hash: &[u8], curve: TWCurve) -> Result<Vec<u8>, Error> {
        fn sign_impl<Key>(signing_key: Key, hash: &[u8]) -> Result<Vec<u8>, Error>
        where
            Key: SigningKeyTrait,
        {
            let hash_to_sign = <Key as SigningKeyTrait>::SigningHash::try_from(hash)
                .map_err(|_| Error::InvalidSignMessage)?;
            signing_key.sign(hash_to_sign).map(|sig| sig.to_vec())
        }

        match curve {
            TWCurve::Secp256k1 => sign_impl(self.to_secp256k1_privkey()?, hash),
            TWCurve::Starkex => sign_impl(self.to_starkex_privkey()?, hash),
        }
    }

    /// Returns the public key associated with the `self` private key and `ty` public key type.
    pub fn get_public_key_by_type(&self, ty: TWPublicKeyType) -> Result<TWPublicKey, Error> {
        match ty {
            TWPublicKeyType::Secp256k1 => {
                let privkey = self.to_secp256k1_privkey()?;
                Ok(TWPublicKey::Secp256k1(privkey.public()))
            },
            TWPublicKeyType::Secp256k1Extended => {
                let privkey = self.to_secp256k1_privkey()?;
                Ok(TWPublicKey::Secp256k1Extended(privkey.public()))
            },
            TWPublicKeyType::Starkex => {
                let privkey = self.to_starkex_privkey()?;
                Ok(TWPublicKey::Starkex(privkey.public()))
            },
        }
    }

    /// Tries to convert [`TWPrivateKey::key`] to [`secp256k1::PrivateKey`].
    fn to_secp256k1_privkey(&self) -> Result<secp256k1::PrivateKey, Error> {
        secp256k1::PrivateKey::try_from(self.key().as_slice())
    }

    /// Tries to convert [`TWPrivateKey::key`] to [`starkex::PrivateKey`].
    fn to_starkex_privkey(&self) -> Result<starkex::PrivateKey, Error> {
        starkex::PrivateKey::try_from(self.key().as_slice())
    }
}
