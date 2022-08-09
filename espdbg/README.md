# esp-serial-dbg command line utility

## Usage

```
espdbg
esp-serial-dbg CLI

USAGE:
    espdbg.exe <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    cli     Developer's CLI
    gdb     GDB Server
    help    Print this message or the help of the given subcommand(s)
```

You need to give the chip and optionally the serial port to `cli` and `gdb` command.

`cli` is for developers of this code and most probably not useful for end-users.

`gdb` starts a GDB server on port 9001.

## GDB

You can use the GDB command line application suitable for your target (e.g. `riscv32-esp-elf-gdb target\riscv32imac-unknown-none-elf\debug\c3_dbg_tst -ex "target remote :9001"` or `xtensa-esp32s3-elf-gdb target\xtensa-esp32s3-none-elf\debug\esp32s3_example -ex "target remote :9001"`) or Visual Studio Code with the Native Debug plugin.

_Native Debug 0.26.0 doesn't work - 0.25.1 works better._

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
