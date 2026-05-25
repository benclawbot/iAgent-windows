#[tokio::main]
async fn main() -> anyhow::Result<()> {
    iagent::desktop_ambient::run(false).await
}
