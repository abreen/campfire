# Alpine CLI Campfire Tutorial

This example shows how a project can give everyone the same small CLI
environment with the same tool versions in a few commands.

The example uses a pinned Alpine Linux image:

- Alpine release: `3.20.10`
- BusyBox: `1.36.1`
- Image: `docker.io/library/alpine@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc`

BusyBox provides common CLI tools such as `sh`, `cat`, `grep`, `sed`, `awk`,
`printf`, and `wget`.

## Requirements

- `cf` installed on your `PATH`
- Podman installed and running

On macOS, start the Podman machine first if it is not already running:

```sh
podman machine start
```

## Start Here

From the Campfire repository root:

```sh
cd examples/alpine-cli
cf check
```

`cf check` validates that Podman works, the pinned image is usable, and the
expected Alpine and BusyBox versions are available. It should finish with:

```text
Campfire check passed
```

## Run a One-Off Command

Ask the campfire what versions it has:

```sh
cf run -- sh -lc 'cat /etc/alpine-release && busybox | head -n 1'
```

Expected output includes:

```text
3.20.10
BusyBox v1.36.1
```

## Read a Project File

The current directory is mounted read-write at `/workspace` inside the
container. Read the example file from inside the campfire:

```sh
cf run -- cat /workspace/message.txt
```

## Write Back to the Host Workspace

Create a file from inside the campfire:

```sh
cf run -- sh -lc 'printf "written from the campfire\n" > /workspace/generated.txt'
cat generated.txt
```

The `cat` command runs on your host. It can read `generated.txt` because the
campfire wrote through the `/workspace` mount.

## Pipe Input Into the Campfire

`cf run` keeps stdin open, so host pipelines work:

```sh
printf 'hello through stdin\n' | cf run -- sh -lc 'cat > /workspace/from-stdin.txt'
cat from-stdin.txt
```

## Enter the Shell

For an interactive session:

```sh
cf enter
```

Inside the shell, try:

```sh
pwd
cat /etc/alpine-release
busybox | head -n 1
cat /workspace/message.txt
exit
```

## Clean Up Generated Files

The tutorial creates files in this example directory. Remove them with:

```sh
rm -f generated.txt from-stdin.txt
```
