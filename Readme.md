# stampit

Run `main.rs` as a one-off debug version

```sh
cargo run -- ~/Desktop/2024_rs/2023-12-31_17.32.54.jpg
```

Compile a distributable binary of the program

```sh
cargo build --release
```

After building, you’ll find the executable here:

```sh
./target/release/stampit <file_or_directory_path>
```

Optionally, you can install the binary to your cargo path `~/.cargo/bin`

```sh
cargo install --path .
```

## changelog

**0.1.1**

- Fix hidden files bug to avoid renaming files like `.DS_Store` on macOS

**0.1.0**

- Initial release
