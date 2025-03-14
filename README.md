# Setup:
1. ```shell
    cargo run
   ```
2. ```shell
    ffplay -f s16le -ar 8000 -i udp://192.168.177.97:54103
   ```

# Note:
- `192.168.177.97` is the client (Windows) address, can be obtained via `cmd:ipconfig`
- `54103` port must match in both `main.rs` and `ffplay` command