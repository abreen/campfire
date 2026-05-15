# AGENTS.md

## Environment Notes

- This project is developed on Fedora Linux Silverblue. Expect an immutable-ish host plus Toolbox/Podman workflows instead of a traditional mutable workstation.
- Prefer `fd`, `rg`, `sg`, `jq`, and `yq` when available. In a bare host shell, `fd` may be missing even if it exists in a toolbox; fall back to `rg --files` when needed.
- The host may have Podman but not Cargo/Rust. The existing development toolbox is named `fedora-toolbox-43`; run Rust commands through it when host Cargo is unavailable:

```sh
toolbox run --container fedora-toolbox-43 cargo fmt --check
toolbox run --container fedora-toolbox-43 cargo clippy --all-targets -- -D warnings
toolbox run --container fedora-toolbox-43 cargo test --all-targets
```

## Windows Host Notes

- A Windows host is available at `intrrupt@pc`. Native Windows is the working Windows acceptance path.
- The default SSH shell on `intrrupt@pc` is `cmd.exe`. Use `cd /d` for drive-aware directory changes.
- Native Windows dependencies installed during bring-up:
  - Rust via `winget install -e --id Rustlang.Rustup`
  - MSVC linker via `winget install -e --id Microsoft.VisualStudio.2022.BuildTools` with the C++ workload
  - Podman via `winget install -e --id RedHat.Podman`
- Native Windows Podman uses a Podman-managed WSL machine. Start it before real integration tests:

```cmd
podman machine start
```

- Native Windows verification command:

```cmd
cd /d %USERPROFILE%\src\campfire
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
podman machine start
set CAMPFIRE_RUN_PODMAN_TESTS=1&& cargo test --test podman_integration_tests -- --nocapture
```

- Native Windows read-only file mounts use WSL-style container destinations. A host file such as `C:\Users\intrrupt\.aws\config` should appear inside the Linux container as `/mnt/c/Users/intrrupt/.aws/config`.
- Fedora WSL is installed as `FedoraLinux-43` and can run the Rust checks:

```cmd
wsl -d FedoraLinux-43 -- bash -lc "cd /mnt/c/Users/intrrupt/src/campfire && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --all-targets"
```

- Do not treat WSL-native Podman as a passing runtime target yet. It can run simple containers with `--network host`, but Campfire's normal Podman invocation currently fails in Fedora WSL with a netavark/nftables error.

## Podman Testing

- Avoid treating nested Podman inside Toolbox as the normal path. It can fail with user-namespace pause-process errors such as `cannot re-exec process to join the existing user namespace`.
- For real runtime behavior, test against host Podman from outside Toolbox when possible.
- If Cargo is only available inside Toolbox but host Podman is needed, use `flatpak-spawn --host podman` through a temporary `podman` wrapper on the test process `PATH`.
- Real Podman integration tests are opt-in:

```sh
CAMPFIRE_RUN_PODMAN_TESTS=1 cargo test --test podman_integration_tests -- --nocapture
```

- On Fedora Silverblue with SELinux enabled, arbitrary project bind mounts may be unreadable inside containers unless Campfire passes `--security-opt label=disable`. Keep tests covering this behavior.
- On native Windows, keep real integration coverage for read-only files, workspace writes, stdin, named commands, and port publishing because all of these have platform-specific failure modes.

## Project Conventions

- This is a private Rust CLI package. Keep `publish = false` in `Cargo.toml`.
- Track `Cargo.lock`; Campfire is an application/binary, not just a library.
- Keep `/target/` ignored. Temporary wrappers, integration-test scratch dirs, and local build products should stay under `target/` when possible.
- CI should mirror the local checks: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-targets`.

## Verification

- Before claiming Rust changes are complete, run:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

- If host Cargo is unavailable, run the same commands via the Toolbox command prefix shown above.
