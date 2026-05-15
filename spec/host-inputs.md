# Host Inputs

Host inputs are the parts of the user's machine that a project chooses to expose
inside the campfire. Campfire only passes selected inputs. It does not mirror the
entire host environment or filesystem.

## Environment Variables

Environment variables are read when a Campfire command starts. A later command
should observe later host values.

Optional variables listed in `env.pass` are copied only when present. Missing
optional variables do not fail validation.

Required variables listed in `env.required` must be present. If any are missing,
Campfire fails before starting a container and reports all missing required
variables it found.

Fixed variables listed in `env.set` are always passed. Fixed variables override
copied variables with the same name.

On Unix-like systems, host environments can contain byte strings that are not
valid UTF-8. Campfire ignores unrelated non-UTF-8 entries rather than crashing.
Configuration names and values remain UTF-8 because `Campfire.toml` is UTF-8
text.

## Files

File inputs are read when a Campfire command starts. A later command should
observe files that were created or removed after an earlier command.

Optional files listed in `files.readonly` are mounted when present and ignored
when absent.

Required files listed in `files.required_readonly` must exist. If any are
missing, Campfire fails before starting a container and reports all missing
required files it found.

The same file should only be mounted once even when it is listed in more than one
place.

## Path Resolution

Host file paths are resolved before validation and before building container
mounts.

- `~` resolves to the host home directory.
- `~/path` resolves under the host home directory.
- Absolute paths stay absolute.
- Relative paths resolve from the project root.

The project root is the directory containing `Campfire.toml`. This matters when
users run `cf` from a subdirectory.

## Privacy and Safety

Campfire should keep host exposure explicit. Projects should ask only for the
environment variables and files they need.

Read-only file mounts are the default for additional host files. This lets tools
read credentials or project-local config without writing back to those host
files.
