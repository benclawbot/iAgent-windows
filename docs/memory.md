# Memory Durability Contract

This document defines persistence behavior for `iagent` memory data.

## Storage Backend

Current persistent memory stores are JSON graph files under the iAgent data directory:

- project-scoped: `memory/projects/<project-hash>.json`
- global: `memory/global.json`
- optional backup sidecars: `*.bak`

Runtime path resolution is managed by `iagent-storage` and `storage::iagent_dir()`.

## Crash and Recovery Behavior

- writes use atomic temp-file replacement semantics to avoid partial writes
- backup files are used as recovery fallback for corrupted primary JSON
- recovery path logs events and restores from backup when possible

## WAL Mode

Not applicable for the current JSON graph backend (no SQLite WAL in this path).

## Export / Portability

CLI command:

- `iagent memory export --output <file> --scope all|project|global`

## Clear / Deletion

CLI command:

- `iagent memory clear --scope all|project|global`

`memory clear` requires explicit confirmation (`YES`) unless `--force` is supplied.

## Schema Versioning and Migrations

- graph files carry `graph_version`
- loader migrates legacy memory store formats to graph format
- migration keeps backups where possible

## Uninstall Expectations

- binaries/shortcuts/startup entries are removable via uninstall script
- user data removal remains opt-in during uninstall (config, logs, memory, runtime artifacts)
