# Note:
`Connect` button on client webui would lead to error, if the controller is running via debugger.

In other words, after running:
```shell
cargo xtask run-example esp-hal esp32c3 embassy_usb_serial_jtag
```
do the `CTRL+C`, otherwise the terminal occupies the serial port.
So the controller needs to be flashed, but not occupied by another JTAG consumer.