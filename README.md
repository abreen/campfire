# Campfire 🏕️

Campfire (`cf`) opens a repo-defined Podman toolbox for a project. The project keeps a
`Campfire.toml` file at its root, and Campfire uses that file to choose the container
image, mount the workspace, pass selected host inputs, and run quick tool checks.

Campfire is a private Rust CLI prototype. It is intentionally not published to
crates.io (`publish = false`).

## What it gives you

- One command to enter the same containerized shell for a repo.
- A read-write project mount inside the container, usually at `/workspace`.
- Optional or required host environment variables and read-only files.
- A `cf check` command that validates Podman, required inputs, and configured tools.
- Reusable project commands, similar to `package.json` scripts, that run inside
  the campfire environment.
- Optional localhost port publishing for dev servers started inside the campfire.

## Requirements

- Podman installed and available on `PATH`.
- Rust/Cargo to build the `cf` binary.
- A container image that contains the tools your project needs.

## Quick start

Build Campfire, create a starter config, then enter the container:

```sh
cargo build
./target/debug/cf init --image docker.io/library/fedora:latest
./target/debug/cf check
./target/debug/cf enter
```

If `cf` is already on your `PATH`, use `cf` directly instead of `./target/debug/cf`.

To try a complete example with pinned CLI tool versions:

```sh
cd examples/alpine-cli
cf check
cf run -- sh -lc 'cat /etc/alpine-release && busybox | head -n 1'
```

Run a one-off command in the Campfire environment:

```sh
cf run -- sh -lc 'echo hello > /workspace/new-file.txt'
```

Run a reusable project command from `Campfire.toml`:

```sh
cf run versions
```

## Commands

| Command | What it does |
| --- | --- |
| `cf init --image IMAGE` | Writes a starter `Campfire.toml` in the current directory. |
| `cf check` | Validates the config, required host inputs, Podman, and configured tool checks. |
| `cf enter` | Starts an interactive shell inside the configured container. |
| `cf run NAME [ARGS...]` | Runs a configured command inside the container. |
| `cf run -- COMMAND ...` | Runs a raw command inside the configured container. |

Campfire looks for `Campfire.toml` in the current directory and then walks upward. The
directory containing `Campfire.toml` is the project root.

## `Campfire.toml`

A compact config can be just an image:

```toml
[campfire]
image = "docker.io/library/fedora:latest"
```

A fuller example:

```toml
[campfire]
image = "ghcr.io/acme/service-tools:2026.05"
shell = "/bin/bash"

[workspace]
path = "/workspace"

[env]
pass = ["AWS_PROFILE", "AWS_REGION"]
required = ["AWS_PROFILE"]
set = { APP_ENV = "dev" }

[files]
readonly = ["~/.aws/config"]
required_readonly = ["~/.aws/credentials"]

[tools.aws]
check = "aws --version"
contains = "aws-cli/2."

[commands.status]
run = "git status"
description = "Show repository status"

[commands.versions]
run = "aws --version"
description = "Show pinned tool versions"

[commands.serve]
run = "python -m http.server 8080 --bind 0.0.0.0"
description = "Start the project dev server"

[[ports]]
container = 8080
```

### Config reference

- `[campfire]`
  - `image` is required.
  - `shell` defaults to `/bin/sh` and is used by `cf enter` and tool checks.
- `[workspace]`
  - `path` defaults to `/workspace`.
  - The project root is mounted read-write at this path.
- `[env]`
  - `pass` copies host variables when they exist.
  - `required` copies host variables and fails early if any are missing.
  - `set` defines fixed values and overrides copied values with the same name.
- `[files]`
  - `readonly` mounts existing files read-only and ignores missing files.
  - `required_readonly` mounts files read-only and fails early if any are missing.
  - `~` expands from the host `HOME`; relative paths resolve from the project root.
- `[tools.NAME]`
  - `check` runs during `cf check` inside the container.
  - `contains` is optional and requires the combined stdout/stderr to contain the text.
- `[commands.NAME]`
  - `run` is a reusable shell command snippet, similar to a `package.json` script
    but executed inside the campfire environment.
  - `description` is optional metadata for humans and future help output.
  - Extra args are appended, so `cf run status -sb` behaves like `git status -sb`.
- `[[ports]]`
  - `container` is the required TCP port inside the container.
  - `host` defaults to the same value as `container`.
  - `bind` defaults to `127.0.0.1` for local-only access.
  - Ports are published for `cf enter` and `cf run`, not for `cf check`.

Read-only files are mounted at the same absolute path inside the container.
Files written under the workspace path are available outside the campfire because
the project root is mounted read-write.

For a configured dev server, run the project command and open the host port:

```sh
cf run serve
curl http://127.0.0.1:8080
```

## How the pieces fit together

- `src/app.rs` owns the CLI commands and error flow.
- `src/config.rs` defines `Campfire.toml` and config discovery.
- `src/host.rs` resolves host environment variables and files.
- `src/podman.rs` builds the `podman run` arguments.
- `src/main.rs` is the small binary entry point.
- `tests/` covers CLI behavior, config parsing, host input resolution, Podman argument
  building, and opt-in real Podman integration tests.

## Specs

Design notes live in `spec/`. They describe intended behavior without tying that
behavior to a specific Rust implementation.

## Development

Run the same checks as CI before opening a PR:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

On Fedora Silverblue, Cargo may only be available inside the project toolbox:

```sh
toolbox run --container fedora-toolbox-43 cargo fmt --check
toolbox run --container fedora-toolbox-43 cargo clippy --all-targets -- -D warnings
toolbox run --container fedora-toolbox-43 cargo test --all-targets
```

Run real Podman integration tests only when host Podman is available:

```sh
CAMPFIRE_RUN_PODMAN_TESTS=1 cargo test --test podman_integration_tests -- --nocapture
```

You can override the integration-test image with `CAMPFIRE_PODMAN_TEST_IMAGE`.

## Troubleshooting

- `could not find Campfire.toml`: run `cf` from the project root or a subdirectory.
- `missing required env vars`: export the variables listed in the error.
- `missing required files`: create the listed files or update `required_readonly`.
- `podman is not installed or not on PATH`: install Podman or update `PATH`.
- `address already in use`: change the configured `host` port or stop the other
  process using it.
- SELinux bind mounts on Fedora Silverblue are expected to work because Campfire passes
  `--security-opt label=disable` to Podman.
