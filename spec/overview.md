# Campfire Overview

Campfire is a project-local command line environment. A repository describes the
container image, shell, workspace mount, host inputs, and tool checks it needs.
Anyone with the repository and a compatible container runtime can enter the same
configured command line environment.

The user-facing command is `cf`. A project opts in by placing `Campfire.toml` at
the project root. Campfire can be run from that root or from any subdirectory
inside the project.

## Goals

- Make a clean CLI environment easy to enter for a repository.
- Keep project setup in a human-readable file that can be committed with code.
- Mount the project workspace read-write inside the environment.
- Pass only selected host environment variables and files into the environment.
- Validate required host inputs and expected tools before a user starts work.
- Keep behavior independent of Git when possible, while leaving room for future
  Git-aware campfire isolation.

## Non-Goals

- Campfire is not a general container orchestrator.
- Campfire is not a replacement for Dockerfiles or image build systems.
- Campfire does not attempt to synchronize every host file or environment
  variable by default.
- Campfire does not own project dependency installation unless the project image
  chooses to include that tooling.
- Campfire does not promise that host and container operating systems are the
  same.

## Mental Model

A campfire is the place where a repository-defined CLI session runs. The host
still owns the repository files, credentials, and environment variables.
Campfire builds a container invocation that makes the selected pieces visible in
the container.

The project workspace is mounted read-write because editing code is the main
workflow. Additional files are mounted read-only because credentials and host
configuration should be available without being mutated by container tools.

## Refresh Model

Campfire reads host state each time a command starts. If a user changes an
environment variable or creates a required file outside the campfire, a later
`cf check`, `cf enter`, or `cf run` should see that new host state.

Campfire does not keep a long-lived cache of host inputs.

## Compatibility

Campfire should behave consistently on Linux and macOS for supported workflows.
Path spelling can differ across platforms, especially on macOS where temporary
directories can resolve through `/private/var`. Behavior should follow canonical
host paths when the operating system presents them.
