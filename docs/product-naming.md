# Product Naming

iAgent is the user-facing product name. The executable, Rust crate names, Python
package name, config directory, and environment variable prefix remain
lowercase/uppercase `iagent` and `IAGENT_*` for predictable CLI and filesystem
behavior.

## User-Facing Text

Use `iAgent` in:
- README and docs prose
- installer and launcher labels
- setup/login prompts
- desktop notifications
- provider/menu labels when referring to the product

Use lowercase `iagent` in:
- commands, for example `iagent login --provider openai`
- paths, for example `~/.iagent/config.toml`
- package, crate, binary, and module identifiers
- environment variables, for example `IAGENT_HOME`

## Compatibility

Do not reintroduce old `J-Code` or `Clicky` names in product-facing text. If a
historical compatibility note is required, keep it explicit and scoped to that
legacy migration context.
