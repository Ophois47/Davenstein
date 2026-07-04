# Davenstein

A Wolfenstein 3-D recreation written in Rust with the Bevy engine

## Note

Left Control `LCtrl` releases the mouse from the window

## Build

### Linux

Build the release executable and rebuild `assets.pak` into `target/release`

```bash
./build_create_assets.sh
```

Or build manually with:

```bash
cargo update && cargo build --release
cargo run --bin pak_builder --release -- --root assets --out target/release/assets.pak
```

### Windows PowerShell

Build the release executable and rebuild `assets.pak` into `target\release`

```powershell
.\build_create_assets.ps1
```

If PowerShell blocks the script, run it once with:

```powershell
powershell -ExecutionPolicy Bypass -File .\build_create_assets.ps1
```

## Cross Compilation

Cross-compiling requires a container engine, either Podman or Docker, and the `cross` tool

```bash
cargo install cross --git https://github.com/cross-rs/cross
```

On Fedora, or any Podman host, tell `cross` to use Podman

```bash
export CROSS_CONTAINER_ENGINE=podman
```

### Windows GNU

```bash
cross build --release --target x86_64-pc-windows-gnu --target-dir target/win
```

### Linux ARMv7 GNU

```bash
cross build --release --target armv7-unknown-linux-gnueabihf --target-dir target/arm
```

## Assets Pak

### Build or rebuild `assets.pak`

```bash
cargo run --bin pak_builder --release -- --root assets --out dist/assets.pak
```

### Build or rebuild `assets.pak` in the release directory

```bash
cargo run --bin pak_builder --release -- --root assets --out target/release/assets.pak
```

## Bugs

- Cross-platform support needs to be revisited
- Mouse capture needs to happen on program start
- Make god mode activate with the canonical `M + I + L` command
- Change View size does not respect menu UI
