use std::io::Write as _;
use std::sync::Arc;

use dissedoff_client_core::{GatewayClient, RestClient};
use dissedoff_protocol::EventType;
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() {
    let server_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

    out(&format!("Dissedoff CLI — connecting to {server_url}"));
    let client = Arc::new(RestClient::new(server_url.clone()));

    let username = prompt("username: ");
    let password = prompt("password: ");

    let auth = match client.login(username.clone(), password.clone()).await {
        Ok(auth) => auth,
        Err(_) => {
            out(&format!("login failed — registering a new account as '{username}' instead"));
            match client.register(Some(username), password).await {
                Ok(auth) => auth,
                Err(e) => {
                    eprintln!("could not register either: {e}");
                    std::process::exit(1);
                }
            }
        }
    };
    out(&format!(
        "signed in as {} (public id {})",
        auth.username, auth.public_id
    ));

    let channels = client.list_channels().await.unwrap_or_default();
    let channel = if let Some(first) = channels.into_iter().next() {
        first
    } else {
        match client.create_channel("general", "text").await {
            Ok(c) => {
                out(&format!("no channels existed yet — created #{}", c.name));
                c
            }
            Err(e) => {
                eprintln!(
                    "no channels exist and you don't have permission to create one ({e}) — ask the server owner"
                );
                std::process::exit(1);
            }
        }
    };
    out(&format!("-- #{} --", channel.name));

    if let Ok(history) = client.list_messages(&channel.id).await {
        for msg in history.into_iter().rev() {
            out(&format!("{}: {}", msg.author_username, msg.content));
        }
    }

    let token = client.token().await.expect("just authenticated");
    let mut gateway = GatewayClient::connect(&server_url, &token)
        .await
        .expect("failed to connect to gateway");

    let print_task = tokio::spawn(async move {
        while let Some(envelope) = gateway.incoming.recv().await {
            if envelope.t == EventType::MessageCreate {
                let author = envelope.d["author_username"].as_str().unwrap_or("?");
                let content = envelope.d["content"].as_str().unwrap_or("");
                out(&format!("{author}: {content}"));
            }
        }
    });

    out("type a message and press enter to send; Ctrl+D to quit");
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Err(e) = client.create_message(&channel.id, line).await {
            eprintln!("failed to send: {e}");
        }
    }

    print_task.abort();
}

/// `println!` alone is fully buffered (not flushed per line) once stdout
/// isn't a TTY — e.g. piped input/output, as any script or test harness
/// driving this CLI will do — so output can sit invisible until the process
/// exits. Flush explicitly instead.
fn out(line: &str) {
    println!("{line}");
    std::io::stdout().flush().ok();
}

fn prompt(label: &str) -> String {
    print!("{label}");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    line.trim().to_string()
}
