/*
    zk-paillier

    Copyright 2018 by Kzen Networks

    zk-paillier is free software: you can redistribute
    it and/or modify it under the terms of the GNU General Public
    License as published by the Free Software Foundation, either
    version 3 of the License, or (at your option) any later version.

    @license GPL-3.0+ <https://github.com/KZen-networks/zk-paillier/blob/master/LICENSE>
*/

mod wi_dlog_proof;
mod correct_key_ni;

mod errors;
mod utils;

pub use self::{
    correct_key_ni::{NiCorrectKeyProof, SALT_STRING},
    wi_dlog_proof::*,
};

pub use self::{errors::IncorrectProof, utils::compute_digest};
