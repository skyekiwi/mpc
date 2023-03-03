// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const execSync = require('./execSync.cjs');
console.log('$ yarn pack-wasm', process.argv.slice(2).join(' '));

function startAll() {
    execSync(`cargo build -p skw-mpc-node-bin --release`);
    execSync(`cargo build -p skw-mpc-client-bin --release`);
    execSync(`cargo build -p skw-auth-service --release`);
}

startAll()