# Rebrand Inventory: `jcode` -> `iAgent`

## Summary
This repository still contains many internal/user-facing `jcode` references.
Priority should be user-facing strings and runtime paths first, then deeper crate/internal symbols.

## Highest-Concentration Files (sample)

From a repo-wide search snapshot:

1. `Cargo.toml` (workspace and crate names)
2. `crates/jcode-storage/src/lib.rs`
3. `src/process_title.rs`
4. `src/setup_hints.rs`
5. `src/provider_catalog.rs`
6. `src/provider/openrouter.rs`

## Migration Phases

1. User-facing polish first
- CLI help text, startup banners, error messages.
- UI labels and docs/examples.
- Default config/log paths.

2. Compatibility bridge
- Keep `JCODE_*` env vars supported.
- Add `IAGENT_*` env vars as preferred.
- Emit deprecation warnings when old names are used.

3. Internal rename pass
- Module/crate names and internal symbols.
- Test fixtures and snapshots.

## Suggested First PR

1. Add alias env handling (`JCODE_*` + `IAGENT_*`).
2. Rename user-visible app strings to `iAgent`.
3. Update install/runtime path messaging to mention `~/.iagent` while still reading `~/.jcode`.

## Compatibility Shim (Current)

- `IAGENT_*` environment overrides are accepted and mapped to existing `JCODE_*` config keys.
- `IAGENT_HOME` is mapped to `JCODE_HOME` when `JCODE_HOME` is not set.
- Existing `JCODE_*` names continue to work during migration.

## Migration Guidance

1. Prefer setting `IAGENT_*` variables in new docs and deployment manifests.
2. Keep `JCODE_*` support during the compatibility window.
3. Remove old names only after a documented sunset release.
