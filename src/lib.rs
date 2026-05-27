#![allow(
    dead_code,
    non_camel_case_types,
    unknown_lints,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::manual_checked_ops,
    clippy::manual_ok_err,
    clippy::manual_range_contains,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_lifetimes,
    clippy::never_loop,
    clippy::new_without_default,
    clippy::redundant_closure,
    clippy::too_many_arguments,
    clippy::unnecessary_cast,
    clippy::unnecessary_lazy_evaluations,
    clippy::unnecessary_sort_by,
    clippy::unwrap_or_default,
    clippy::useless_format,
    clippy::useless_conversion
)]
#![recursion_limit = "512"]

#[macro_use]
pub mod logging;

pub mod agent;
pub mod ambient;
pub mod ambient_runner;
pub mod ambient_scheduler;
pub mod attention_budget;
pub mod auth;
pub mod background;
pub mod browser;
pub mod build;
pub mod bus;
pub mod cache_tracker;
pub mod catchup;
pub mod channel;
pub mod cli;
pub mod compaction;
pub mod config;
pub mod connector_packs;
pub mod copilot_usage;
pub mod core_loop_metrics;
pub mod desktop_ambient;
pub mod dictation;
#[cfg(feature = "embeddings")]
pub mod embedding;
#[cfg(not(feature = "embeddings"))]
pub mod embedding_stub;
#[cfg(not(feature = "embeddings"))]
pub use embedding_stub as embedding;
pub mod env;
pub mod gateway;
pub mod gmail;
pub mod goal;
pub mod goal_judge;
pub mod id;
pub mod import;
pub mod intent_manifest;
pub mod login_qr;
pub mod mcp;
pub mod meeting_memory;
pub mod memory;
pub mod memory_agent;
pub mod memory_graph;
pub mod memory_log;
pub mod memory_types;
pub mod message;
pub mod network_retry;
pub mod notifications;
pub mod overnight;
pub mod perf;
pub mod personal_daemon;
pub mod personal_layer;
pub mod plan;
pub mod platform;
pub mod proactive_briefings;
pub mod process_memory;
pub mod process_title;
pub mod processing_report;
pub mod prompt;
pub mod protocol;
pub mod provider;
pub mod provider_catalog;
pub mod recipe_catalog;
pub mod registry;
pub mod remote_dispatch;
pub mod restart_snapshot;
pub mod runtime_memory_log;
pub mod safety;
pub mod server;
pub mod session;
pub mod setup_hints;
pub mod side_panel;
pub mod sidecar;
pub mod skill;
pub mod soft_interrupt_store;
pub mod ssh_remote;
pub mod startup_profile;
pub mod stdin_detect;
pub mod storage;
pub mod subscription_catalog;
pub mod telegram;
pub mod telemetry;
pub mod terminal_launch;
pub mod think_filter;
pub mod todo;
pub mod tool;
pub mod transport;
pub mod update;
pub mod usage;
pub mod util;

use anyhow::Result;
use std::sync::Mutex;

static CURRENT_SESSION_ID: Mutex<Option<String>> = Mutex::new(None);

pub fn set_current_session(session_id: &str) {
    if let Ok(mut guard) = CURRENT_SESSION_ID.lock() {
        *guard = Some(session_id.to_string());
    }
}

pub fn get_current_session() -> Option<String> {
    CURRENT_SESSION_ID.lock().ok()?.clone()
}

pub async fn run() -> Result<()> {
    cli::startup::run().await
}
