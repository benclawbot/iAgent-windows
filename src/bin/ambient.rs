#[tokio::main]
async fn main() -> anyhow::Result<()> {
    jcode::desktop_ambient::run(false).await
}

