use super::args::{Args, Command};
use anyhow::Result;
use std::io::{self, IsTerminal, Write};

struct ProviderBootstrap {
    id: &'static str,
    label: &'static str,
    signup_url: &'static str,
    env_file: &'static str,
    env_key: &'static str,
}

const PROVIDERS: &[ProviderBootstrap] = &[
    ProviderBootstrap {
        id: "openai",
        label: "OpenAI",
        signup_url: "https://platform.openai.com/signup",
        env_file: "openai.env",
        env_key: "OPENAI_API_KEY",
    },
    ProviderBootstrap {
        id: "openrouter",
        label: "OpenRouter",
        signup_url: "https://openrouter.ai/",
        env_file: "openrouter.env",
        env_key: "OPENROUTER_API_KEY",
    },
    ProviderBootstrap {
        id: "gemini",
        label: "Gemini",
        signup_url: "https://aistudio.google.com/",
        env_file: "gemini.env",
        env_key: "GEMINI_API_KEY",
    },
];

pub(crate) fn maybe_run_first_run_setup(args: &Args) -> Result<()> {
    if !should_run_for_command(args) {
        return Ok(());
    }

    let Some(config_path) = crate::config::Config::path() else {
        return Ok(());
    };
    if config_path.exists() {
        return Ok(());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    eprintln!();
    eprintln!("No iAgent config found. Running first-run setup...");
    eprintln!("Config location: {}", config_path.display());
    eprintln!();

    let provider = prompt_provider()?;
    eprintln!("{} signup: {}", provider.label, provider.signup_url);
    let api_key = prompt("API key (input is visible): ")?;

    let mut config = crate::config::Config::default();
    config.provider.default_provider = Some(provider.id.to_string());
    config.save()?;

    if !api_key.trim().is_empty() {
        super::provider_init::save_named_api_key(
            provider.env_file,
            provider.env_key,
            api_key.trim(),
        )?;
    }

    run_self_check(provider, !api_key.trim().is_empty())?;
    eprintln!();
    eprintln!("First-run setup complete.");
    eprintln!();
    Ok(())
}

fn should_run_for_command(args: &Args) -> bool {
    matches!(
        args.command,
        None | Some(Command::Run { .. }) | Some(Command::Repl) | Some(Command::Serve { .. })
    )
}

fn prompt_provider() -> Result<&'static ProviderBootstrap> {
    eprintln!("Choose a provider:");
    for (index, provider) in PROVIDERS.iter().enumerate() {
        eprintln!("  {}) {}", index + 1, provider.label);
    }

    loop {
        let response = prompt("Selection [1-3, default 1]: ")?;
        let trimmed = response.trim();
        if trimmed.is_empty() {
            return Ok(&PROVIDERS[0]);
        }
        if let Ok(num) = trimmed.parse::<usize>()
            && (1..=PROVIDERS.len()).contains(&num)
        {
            return Ok(&PROVIDERS[num - 1]);
        }
        eprintln!("Invalid selection. Enter 1, 2, or 3.");
    }
}

fn prompt(label: &str) -> Result<String> {
    let mut stdout = io::stdout();
    write!(stdout, "{label}")?;
    stdout.flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value)
}

fn run_self_check(provider: &ProviderBootstrap, has_key: bool) -> Result<()> {
    eprintln!();
    eprintln!("Running self-check...");
    let config_exists = crate::config::Config::path()
        .as_ref()
        .map(|path| path.exists())
        .unwrap_or(false);
    eprintln!(
        "  config.toml: {}",
        if config_exists { "ok" } else { "missing" }
    );

    if has_key {
        let loaded = crate::provider_catalog::load_api_key_from_env_or_config(
            provider.env_key,
            provider.env_file,
        )
        .is_some();
        eprintln!(
            "  {}: {}",
            provider.env_key,
            if loaded { "ok" } else { "missing" }
        );
    } else {
        eprintln!("  API key: skipped (none entered)");
    }

    Ok(())
}
