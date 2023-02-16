// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const execSync = require('./execSync.cjs');
console.log('$ yarn copy:examples', process.argv.slice(2).join(' '));

function copyExamples() {
  execSync('PATH="/opt/homebrew/opt/llvm/bin:$PATH" CC=/opt/homebrew/opt/llvm/bin/clang AR=/opt/homebrew/opt/llvm/bin/llvm-ar cd crates/skw-mpc-wasm && yarn && yarn serve');
}

copyExamples()
