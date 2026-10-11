use dissedoff_server::config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg = Config::from_env();
    let (app, _state) = dissedoff_server::build_app(&cfg).await;

    tracing::info!("dissedoff-server listening on {}", cfg.bind_addr);
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr)
        .await
        .expect("failed to bind listen address");
    axum::serve(listener, app).await.expect("server error");
}
