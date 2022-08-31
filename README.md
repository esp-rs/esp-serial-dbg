# esp-serial-dbg - debugging of esp-hal based applications via serial

## About

_This is experimental! At least the contained examples should work fine on ESP32, ESP32-S2, ESP32-S3 and ESP32-C3._

This is a way to do some basic debugging via a serial connection on ESP32, ESP32-S3,ESP32-C3 and ESP32-S2 without additional hardware when developing code with `esp-hal`.

## Basic Usage

- make sure you have the GDB command line application suitable for your target installed and in your system-path
- add the `esp-serial-dbg` dependency to your binary crate (`esp-serial-dbg = { package = "esp-serial-dbg", git = "https://github.com/bjoernQ/esp-serial-dbg.git" }`)
- initialize the library early in your `main.rs`(`esp_serial_dbg::init(Serial::new(peripherals.UART0));`)
- flash your application as usual
- run the `espdbg` command line utility (e.g. `espdbg gdb esp32s3` and optionally pass the serial port to use)
- make sure you have installed the _Native Debug_ extension in version 0.25.1 in Visual Studio Code (version 0.26.0 does NOT work)
- add a launch configuration similar to those contained in the examples
- launch a debug session

If something goes wrong you should restart the target and restart `espdbg`.

`espdbg` uses the log crate and env logger - you can get a lot of information by setting the env-var `RUST_LOG` to `info` or `trace`

While this enabled basic debugging the general approach has some technical limitations!
If you need more advanced debugging you should use a JTAG debugger with OpenOCD or probe-rs (probe-rs currently only supports ESP32-C3)

Debugging via JTAG/Serial is NOT supported (i.e. the ESP32-C3-DevKit-RUST-1 is NOT supported)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
