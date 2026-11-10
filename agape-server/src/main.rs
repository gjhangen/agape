use agape_server::*;

#[tokio::main]
async fn main() {
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let data = std::path::PathBuf::from(env("AGAPE_DATA").unwrap_or_else(|| "data".into()));
    // agape-server restore backup.json: put a downloaded backup into an empty data folder, then stop
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("restore") {
        let file = args.get(2).expect("agape-server restore <backup.json>");
        let b: serde_json::Value = serde_json::from_slice(&std::fs::read(file).expect("read the backup")).expect("a backup file");
        std::fs::create_dir_all(&data).unwrap();
        match ops::restore(&b, &data) { Ok(n) => { println!("restored {n} files into {}", data.display()); return; } Err(e) => { eprintln!("{e}"); std::process::exit(1); } }
    }
    let speed: f64 = env("AGAPE_SPEED").and_then(|s| s.parse().ok()).unwrap_or(0.1);
    let app = app(&data, speed, env("AGAPE_ADMIN_TOKEN"));
    if let Some(p) = env("AGAPE_DEMO_PASSWORD") { demo_accounts(&app, &p); }
    recover(&app).await;
    // a finished night makes way for the next, so the venue is always open
    let roller = app.clone();
    tokio::spawn(async move { loop { tokio::time::sleep(std::time::Duration::from_secs(20)).await; roll_finished(&roller).await; } });
    let mailer = app.clone();
    tokio::spawn(async move { loop { tokio::time::sleep(std::time::Duration::from_secs(120)).await; retry_mail(&mailer).await; } });
    // a copy of everything once a day, and a check every minute
    let keeper = app.clone();
    tokio::spawn(async move { loop { match ops::snapshot(&keeper.data_dir()) { Ok(Some(d)) => println!("backup: {}", d.display()), Ok(None) => {}, Err(e) => eprintln!("backup failed: {e}") } tokio::time::sleep(std::time::Duration::from_secs(3600)).await; } });
    let carder = app.clone();
    tokio::spawn(async move { loop { tokio::time::sleep(std::time::Duration::from_secs(15)).await; settle_cards(&carder).await; } });
    let watcher = app.clone();
    tokio::spawn(async move { let mut was = None; loop { tokio::time::sleep(std::time::Duration::from_secs(60)).await; watch(&watcher, &mut was).await; } });
    let port = env("PORT").unwrap_or_else(|| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    println!("agape server on :{port} · data in {} · clock {speed} ticks/s", data.display());
    axum::serve(listener, router(app, env("AGAPE_STATIC"))).await.unwrap();
}
