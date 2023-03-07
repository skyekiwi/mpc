// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const fs = require('fs');
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

    fs.writeFileSync('./.env', `CLIENT_OAUTH_SECRET = ${util.u8aToHex(clientSecret)} \n`, {flag: 'a'});
    fs.writeFileSync('./.env', `OWNERSHIP_PROOF_KEY = ${util.u8aToHex(ownershipProofKey)} \n`, {flag: 'a'});
    fs.writeFileSync('./.env', `OWNERSHIP_VERIFY_KEY = ${util.u8aToHex(ownershipPublicKey)} \n`, {flag: 'a'});
    fs.writeFileSync('./.env', `USAGE_CERT_KEY = ${util.u8aToHex(usageCertKey)} \n`, {flag: 'a'});
    fs.writeFileSync('./.env', `USAGE_VERIFY_KEY = ${util.u8aToHex(usageCertPublicKey)} \n`, {flag: 'a'});
    
    console.log(".env for client-side")
    console.log(`NEXTAUTH_SECRET = ${util.u8aToHex(clientSecret)}`);
}

genEnv()
