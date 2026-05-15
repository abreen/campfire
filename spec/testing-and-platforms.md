# Testing and Platforms

Campfire should be tested at three levels: pure behavior, CLI behavior with fake
Podman, and opt-in behavior with real Podman.

## Pure Behavior Tests

Pure behavior tests should cover configuration parsing, config discovery, host
input resolution, and Podman argument construction. These tests should not need
Podman or network access.

These tests are the fastest place to document exact rules such as path
resolution, environment precedence, and run mode flags.

## CLI Tests with Fake Podman

CLI tests should execute the `cf` binary with temporary projects. A fake `podman`
command can record arguments and simulate tool output.

These tests should cover:

- command success and failure behavior,
- early validation before Podman runs,
- environment refresh across invocations,
- file refresh across invocations,
- configured port publishing flags for user-facing runs,
- generated Podman arguments that matter to users.

Fake Podman tests should not try to prove that real containers work.

## Real Podman Integration Tests

Real Podman tests are opt-in because they require a working container runtime and
can pull images.

They should cover behavior that fake Podman cannot prove:

- workspace writes reach the host,
- host env and read-only files are visible inside the container,
- `cf run` preserves stdin through the real container process,
- project-root-relative files work when `cf` is run from a subdirectory.
- a server inside `cf run` is reachable from the host through a configured
  published localhost port.

The default integration image should be small and available on common platforms.
Projects can override the image when needed.

## Linux Expectations

Linux development should support Fedora Silverblue. Rust tooling may run inside
Toolbox while Podman runs on the host. Tests can use a temporary Podman wrapper
when the test process needs host Podman from inside Toolbox.

SELinux bind mount behavior is part of the supported Linux workflow.

## macOS Expectations

macOS development should support Apple Silicon with Rust installed through
rustup and Podman installed through Homebrew.

Podman may require a running Podman machine. Campfire treats that as Podman setup
outside the project.

macOS can present temporary paths through canonical `/private/var` paths. Tests
and specs should describe behavior in terms of resolved host paths rather than
assuming Linux path spelling.

## Cross-Platform Verification

Before treating behavior as stable, run the normal Rust checks and the opt-in
Podman integration tests on Linux and macOS when both machines are available.
