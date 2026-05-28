use std::time::{Duration, Instant};

use iagent_desktop_monitor::{ContextType, DesktopMonitor, DesktopMonitorResult};

#[tokio::main(flavor = "current_thread")]
async fn main() -> DesktopMonitorResult<()> {
    let monitor = DesktopMonitor::new()?;
    let mut contexts = monitor.start_monitoring().await;
    let started = Instant::now();

    println!("Desktop monitor started. Listening for 30 seconds...");
    while started.elapsed() < Duration::from_secs(30) {
        if let Some(context) = contexts.recv().await {
            let text_len = context
                .text_content
                .as_deref()
                .map(str::chars)
                .map(Iterator::count)
                .unwrap_or(0);

            println!(
                "[{}] {} - \"{}\" - {} chars",
                label(context.context_type),
                context.app_name,
                context.window_title,
                text_len
            );
        } else {
            break;
        }
    }

    Ok(())
}

fn label(kind: ContextType) -> &'static str {
    match kind {
        ContextType::Email => "Email",
        ContextType::Document => "Document",
        ContextType::Presentation => "Presentation",
        ContextType::Code => "Code",
        ContextType::Chat => "Chat",
        ContextType::Browser => "Browser",
        ContextType::Unknown => "Unknown",
    }
}
