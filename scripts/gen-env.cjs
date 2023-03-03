// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const execSync = require('./execSync.cjs');
const { secureGenerateRandomKey } = require('@skyekiwi/crypto');
const util = require("@skyekiwi/util");
const tweetnacl = require("tweetnacl");
console.log('$ yarn gen-env', process.argv.slice(2).join(' '));

function genEnv() {
    // generate two pair of Ed25519 keys
    const ownershipProofKey = secureGenerateRandomKey();
    const usageCertKey = secureGenerateRandomKey();

    const ownershipPublicKey = tweetnacl.sign.keyPair.fromSeed(ownershipProofKey).publicKey;
    const usageCertPublicKey = tweetnacl.sign.keyPair.fromSeed(usageCertKey).publicKey;

    const clientSecret = secureGenerateRandomKey();

    console.log("1. .env skw-auth-service");
    console.log(`CLIENT_OAUTH_SECRET = ${util.u8aToHex(clientSecret)}`);
    console.log(`OWNERSHIP_PROOF_KEY = ${util.u8aToHex(ownershipProofKey)}`);
    console.log(`OWNERSHIP_VERIFY_KEY = ${util.u8aToHex(ownershipPublicKey)}`);
    console.log(`USAGE_CERT_KEY = ${util.u8aToHex(usageCertKey)}`);
    console.log(`USAGE_VERIFY_KEY = ${util.u8aToHex(usageCertPublicKey)}`);

    console.log("3. .env for client-side")
    console.log(`NEXTAUTH_SECRET = ${util.u8aToHex(clientSecret)}`);
}

genEnv()
