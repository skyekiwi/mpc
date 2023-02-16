/*
    This file is part of Curv library
    Copyright 2018 by Kzen Networks
    (https://github.com/KZen-networks/curv)
    License MIT: https://github.com/KZen-networks/curv/blob/master/LICENSE
*/
mod ext;

#[cfg(feature = "hash_merkle_tree")]
pub mod merkle_tree;

pub use digest::Digest;
pub use ext::*;
