// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const fs = require('fs');
const execSync = require('./execSync.cjs');
console.log('$ yarn start-all', process.argv.slice(2).join(' '));

function startAll() {
    execSync(`cargo build -p skw-mpc-node-bin --release`);
    execSync(`cargo build -p skw-mpc-client-bin --release`);
    execSync(`cargo build -p skw-auth-service --release`);
    
    if (!fs.existsSync('./.env.c')) {
      execSync(`touch .env.c`);
    }

    execSync(`touch .env`);
    execSync(`rm .env`);
    execSync(`touch .env`);

    execSync("yarn gen-env");

    execSync(`touch .env.peers`);
    try {
      execSync(`rm .env.peers`);
    } catch(e) {
      // file not exist
      // nop
    }
    execSync(`touch .env.peers`);

    // Mprocs land

    execSync(`mprocs \
      "RUST_LOG=info cargo run -p skw-mpc-node-bin --release" \
      "RUST_LOG=info cargo run -p skw-mpc-client-bin --release" \
      "RUST_LOG=info cargo run -p skw-auth-service --release"
    `);
}

startAll()