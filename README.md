## MPC

Most of the codebase as of now is forked from ZenGo's MPC impl.

PATH="/opt/homebrew/opt/llvm/bin:$PATH" CC=/opt/homebrew/opt/llvm/bin/clang AR=/opt/homebrew/opt/llvm/bin/llvm-ar cargo build -p skw-mpc-protocol --release --target wasm32-unknown-unknown

## License

The entire code within this repository is licensed under the [GPLv3](LICENSE).

Please [contact us](https://skye.kiwi) if you have questions about
the licensing of our products.
