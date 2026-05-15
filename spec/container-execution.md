# Container Execution

Campfire translates project configuration and host inputs into Podman runs. The
result should feel like entering a project-specific CLI environment, not like
hand-writing container flags for every command.

## Shared Execution Behavior

All container commands use the configured image, workspace path, host inputs, and
read-only file mounts.

The project root is mounted read-write at the configured workspace path. This is
the main editing surface.

Additional files are mounted read-only at their resolved host paths.

Selected environment variables and fixed values are passed explicitly.

Campfire should fail early before running Podman when required host inputs are
missing.

## `cf check`

`cf check` validates that:

- configuration can be discovered and parsed,
- required host inputs are present,
- Podman is available,
- each configured tool check succeeds,
- each configured expected output string is present when specified.

Tool checks run inside the configured image using the configured shell with
shell command mode. Checks should not allocate a TTY and do not need stdin.

## `cf enter`

`cf enter` starts an interactive shell inside the campfire environment.

It uses the configured shell and allocates an interactive TTY. This is the normal
human workflow for using project tools.

## `cf run`

`cf run -- COMMAND ...` runs one command inside the campfire environment.

It keeps stdin open without allocating a TTY. This supports shell pipelines such
as:

```sh
printf 'input' | cf run -- cat
```

The command's exit status should become the `cf run` exit status when available.

## Podman Assumptions

Podman must be available on `PATH` when a Campfire command needs to run a
container.

On Fedora Silverblue and other SELinux systems, project bind mounts need to work
without requiring manual relabeling of arbitrary repositories. Campfire includes
the Podman security option needed for this workflow.

On macOS, Podman normally runs through a Podman machine. Campfire expects the
`podman` command to hide that detail.
