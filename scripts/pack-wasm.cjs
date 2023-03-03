// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const fs = require('fs');
const pako = require("pako");
const execSync = require('./execSync.cjs');
console.log('$ yarn pack-wasm', process.argv.slice(2).join(' '));

function copyExamples() {
  execSync(`cargo build -p skw-mpc-wasm --target wasm32-unknown-unknown --release`);
  execSync(`./wasm-bindgen ./target/wasm32-unknown-unknown/release/skw_mpc_wasm.wasm --out-dir crates/skw-mpc-wasm/build-wasm/ --target web`);
  execSync(`wasm-opt crates/skw-mpc-wasm/build-wasm/skw_mpc_wasm_bg.wasm -Oz -o crates/skw-mpc-wasm/build-wasm/skw_mpc_wasm_opt.wasm`);

  // Cleanup
  execSync(`rm crates/skw-mpc-wasm/build-wasm/skw_mpc_wasm_bg.wasm`);
  execSync(`rm crates/skw-mpc-wasm/build-wasm/skw_mpc_wasm_bg.wasm.d.ts`);

}

copyExamples()
