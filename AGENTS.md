# AGENTS.md

## Build environment

This project is developed inside WSL but targets Windows.

The default Cargo target is intentionally configured as
`x86_64-pc-windows-gnu` in `.cargo/config.toml`.

Use:

```bash
cargo build
cargo build --release
```

Do not remove the Windows GNU target configuration or switch the project
to the native Linux target. Native WSL builds may fail while compiling
the `windows` crate and its dependencies.

The Windows executable is generated under:

`target/x86_64-pc-windows-gnu/{debug,release}/`