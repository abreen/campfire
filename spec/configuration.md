# Configuration

Campfire projects are configured with a `Campfire.toml` file at the project
root. The file should be small enough to review and specific enough to reproduce
the intended CLI environment.

Campfire discovers configuration by starting at the current directory and walking
upward until it finds `Campfire.toml`. The directory containing that file is the
project root.

## Campfire Section

The `campfire` section chooses the container image and shell.

- `image` is required.
- `shell` is optional and defaults to `/bin/sh`.

The shell is used for interactive entry and for tool checks. Projects that use
images without `/bin/sh` should set `shell` to a shell that exists in the image.

## Workspace Section

The `workspace` section chooses where the project root appears inside the
container.

- `path` is optional and defaults to `/workspace`.

The project root is mounted read-write at this path. Commands that write under
the workspace path write through to the host project.

## Environment Section

The `env` section describes host and fixed environment values.

- `pass` copies a host variable when it exists.
- `required` copies a host variable and fails before running Podman when it is
  missing.
- `set` defines fixed values from the configuration file.

When the same name appears in copied values and fixed values, the fixed value
wins. This lets a project set stable defaults while still allowing required host
inputs for secrets or account selection.

## Files Section

The `files` section describes host files that should be mounted read-only.

- `readonly` mounts files that exist and ignores files that are missing.
- `required_readonly` mounts files that exist and fails before running Podman
  when any are missing.

Paths beginning with `~/` are resolved from the host home directory. Absolute
paths are used as absolute host paths. Relative paths are resolved from the
project root, not from the shell's current subdirectory.

Read-only files appear inside the container at the resolved host path on Linux
and macOS. On native Windows, drive paths appear at the matching WSL-style path:
`C:\Users\alex\.aws\config` becomes `/mnt/c/Users/alex/.aws/config`.

## Tools Section

Each `tools.NAME` section describes a validation command for `cf check`.

- `check` is the command to run in the container.
- `contains` is optional text that must appear in combined stdout and stderr.

Tool checks should be quick. They are meant to answer whether expected tools and
versions are present, not to run a full project test suite.

## Commands Section

Each `commands.NAME` section describes a reusable project command. Commands are
similar to `package.json` scripts, but they execute inside the campfire
environment with the configured image, tools, environment variables, files, and
workspace mount.

- `run` is the shell command snippet.
- `description` is optional human-readable metadata.

Command names must be portable shell function names:
`[A-Za-z_][A-Za-z0-9_]*`.

Extra arguments are appended when a command runs. For example, if `status` runs
`git status`, then `cf run status -sb` runs like `git status -sb`.

## Ports Section

Each `[[ports]]` entry publishes one TCP port from the container to the host for
`cf enter` and `cf run`.

- `container` is required and must be between `1` and `65535`.
- `host` is optional, must be between `1` and `65535`, and defaults to the same
  value as `container`.
- `bind` is optional, must be an IPv4 or IPv6 address, and defaults to
  `127.0.0.1`. IPv6 values are written without brackets in `Campfire.toml`,
  such as `bind = "::1"`.

The localhost default keeps dev servers private to the user's machine. Projects
that intentionally need LAN access can set `bind = "0.0.0.0"`.

Ports are not published for `cf check` because tool validation should not fail
just because a developer already has a local server running.

## Example

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
required_readonly = ["~/.aws/credentials", "config/project.env"]

[tools.aws]
check = "aws --version"
contains = "aws-cli/2."

[commands.status]
run = "git status"
description = "Show repository status"

[commands.test]
run = "cargo test"
description = "Run the project test suite inside the campfire"

[commands.serve]
run = "python -m http.server 8080 --bind 0.0.0.0"
description = "Start a local dev server"

[[ports]]
container = 8080
```
