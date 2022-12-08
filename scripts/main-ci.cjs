// Copyright 2021 @skyekiwi authors & contributors
// SPDX-License-Identifier: GPL-3.0-or-later

const execSync = require('./execSync.cjs');
console.log('$ yarn main:ci', process.argv.slice(2).join(' '));

function mainCI() {
  execSync('cargo test --release');
}

mainCI()
