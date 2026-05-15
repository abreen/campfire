# Container Execution

Campfire translates project configuration and host inputs into Podman runs. The
result should feel like entering a project-specific CLI environment, not like
hand-writing container flags for every command.

## Shared Execution Behavior

All container commands use the configured image, workspace path, host inputs, and
read-only file mounts.

The project root is mounted read-write at the configured workspace path. This is
the main editing surface.

Additional files are mounted read-only. On Linux and macOS, the container
destination matches the resolved host path. On native Windows, the container
destination uses the WSL-style path for the same drive location.

Selected environment variables and fixed values are passed explicitly.

Configured ports are published for developer-facing runs. The default bind
address is `127.0.0.1`, so services are reachable from the host machine without
being exposed to the local network unless the project opts into a wider bind
address.

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
Configured ports are not published for tool checks.

## `cf enter`

`cf enter` starts an interactive shell inside the campfire environment.

It uses the configured shell and allocates an interactive TTY. This is the normal
human workflow for using project tools.

Configured ports are published. If a process inside the shell listens on the
configured container port, the user can reach it through the configured host
port.

Configured commands are exposed as shell functions inside the entered shell when
the shell supports startup files. This makes snippets such as `status` or
`versions` available like local aliases.

## `cf run`

`cf run -- COMMAND ...` runs one command inside the campfire environment.

It keeps stdin open without allocating a TTY. This supports shell pipelines such
as:

```sh
printf 'input' | cf run -- cat
```

The command's exit status should become the `cf run` exit status when available.

Configured ports are published. Long-running commands such as dev servers can be
reached from the host while the `cf run` process is still running.

`cf run NAME [ARGS...]` runs a configured command snippet when `NAME` exists in
the config. This is the non-interactive form of the same project commands
exposed inside `cf enter`.

These commands are similar to `package.json` scripts: they give the repository a
shared vocabulary for common tasks. Unlike package scripts, they run inside the
campfire container, so they use the pinned tool environment. Files written under
the workspace path are written through to the host project.

Server processes should listen on the configured container port and on an
address reachable inside the container, such as `0.0.0.0`, when they need to be
reachable from the host through published ports.

## Podman Assumptions

Podman must be available on `PATH` when a Campfire command needs to run a
container.

On Fedora Silverblue and other SELinux systems, project bind mounts need to work
without requiring manual relabeling of arbitrary repositories. Campfire includes
the Podman security option needed for this workflow.

On macOS, Podman normally runs through a Podman machine. Campfire expects the
`podman` command to hide that detail.

On native Windows, Podman normally runs through a Podman-managed WSL machine.
Campfire still invokes the native `podman` command. Read-only file destinations
must be Linux container paths, so Windows drive paths are translated to
`/mnt/<drive>/...` destinations.
