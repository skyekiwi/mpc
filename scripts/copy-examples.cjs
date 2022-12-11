// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const execSync = require('./execSync.cjs');
console.log('$ yarn copy:examples', process.argv.slice(2).join(' '));

function copyExamples() {
  execSync('rm ./bin/*');
  execSync('cp ./target/release/examples/gg20_keygen ./bin');
  execSync('cp ./target/release/examples/gg20_signing ./bin');
  execSync('cp ./target/release/examples/gg20_sm_client ./bin');
  execSync('cp ./target/release/examples/gg20_sm_manager ./bin');
}

copyExamples()
