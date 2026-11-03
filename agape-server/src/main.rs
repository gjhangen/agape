use agape_server::*;

#[tokio::main]
async fn main() {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let data = std::path::PathBuf::from(env("AGAPE_DATA").unwrap_or_else(|| "data".into()));
    let speed: f64 = env("AGAPE_SPEED").and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let app = app(&data, speed, env("AGAPE_ADMIN_TOKEN"));
    if let Some(p) = env("AGAPE_DEMO_PASSWORD") { demo_accounts(&app, &p); }
    recover(&app).await;
    let port = env("PORT").unwrap_or_else(|| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    println!("agape server on :{port} · data in {} · clock {speed} ticks/s", data.display());
    axum::serve(listener, router(app, env("AGAPE_STATIC"))).await.unwrap();
}
