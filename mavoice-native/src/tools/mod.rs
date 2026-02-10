use serde_json::{json, Value};

const MEMORY_DB_PATH: &str = "/home/player3vsgpt/.shieldcortex/memories.db";

/// Execute a tool by name with the given arguments.
/// Returns a JSON value to send back to Gemini as the function response.
pub async fn execute(name: &str, args: &Value) -> Value {
    match name {
        "search_memory" => search_memory(args).await,
        "remember" => remember(args).await,
        "run_command" => run_command(args).await,
        "ask_claude" => ask_claude(args).await,
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

/// Search the ShieldCortex memory database using FTS5.
async fn search_memory(args: &Value) -> Value {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(q) => q.to_string(),
        None => return json!({ "error": "Missing 'query' parameter" }),
    };

    tokio::task::spawn_blocking(move || {
        match rusqlite::Connection::open_with_flags(
            MEMORY_DB_PATH,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(conn) => {
                let sql = "SELECT m.title, m.content, m.category \
                           FROM memories m \
                           JOIN memories_fts f ON m.id = f.rowid \
                           WHERE memories_fts MATCH ?1 \
                           ORDER BY rank \
                           LIMIT 5";
                match conn.prepare(sql) {
                    Ok(mut stmt) => {
                        match stmt.query_map([&query], |row| {
                            Ok(json!({
                                "title": row.get::<_, String>(0).unwrap_or_default(),
                                "content": row.get::<_, String>(1).unwrap_or_default(),
                                "category": row.get::<_, String>(2).unwrap_or_default(),
                            }))
                        }) {
                            Ok(rows) => {
                                let results: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
                                if results.is_empty() {
                                    json!({ "results": [], "message": "No memories found matching that query." })
                                } else {
                                    json!({ "results": results })
                                }
                            }
                            Err(e) => json!({ "error": format!("FTS query failed: {}", e) }),
                        }
                    }
                    Err(e) => json!({ "error": format!("Query failed: {}", e) }),
                }
            }
            Err(e) => json!({ "error": format!("Cannot open memory DB: {}", e) }),
        }
    })
    .await
    .unwrap_or_else(|e| json!({ "error": format!("Task failed: {}", e) }))
}

/// Save a memory to the ShieldCortex database.
async fn remember(args: &Value) -> Value {
    let title = match args.get("title").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return json!({ "error": "Missing 'title' parameter" }),
    };
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => return json!({ "error": "Missing 'content' parameter" }),
    };

    tokio::task::spawn_blocking(move || {
        match rusqlite::Connection::open_with_flags(
            MEMORY_DB_PATH,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ) {
            Ok(conn) => {
                let insert_sql = "INSERT INTO memories (type, category, title, content, project, salience, scope, source) \
                                  VALUES ('long_term', 'note', ?1, ?2, 'maVoice-Linux', 0.6, 'project', 'agent:mavoice')";
                match conn.execute(insert_sql, rusqlite::params![title, content]) {
                    Ok(_) => {
                        let id = conn.last_insert_rowid();
                        // Also insert into FTS index
                        let fts_sql = "INSERT INTO memories_fts (rowid, title, content, tags) VALUES (?1, ?2, ?3, '[]')";
                        let _ = conn.execute(fts_sql, rusqlite::params![id, title, content]);
                        json!({ "success": true, "message": format!("Saved memory: {}", title), "id": id })
                    }
                    Err(e) => json!({ "error": format!("Failed to save: {}", e) }),
                }
            }
            Err(e) => json!({ "error": format!("Cannot open memory DB: {}", e) }),
        }
    })
    .await
    .unwrap_or_else(|e| json!({ "error": format!("Task failed: {}", e) }))
}

/// Run a shell command with a 30-second timeout.
async fn run_command(args: &Value) -> Value {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c.to_string(),
        None => return json!({ "error": "Missing 'command' parameter" }),
    };

    log::info!("[Tool:run_command] Executing: {}", command);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::process::Command::new("bash")
            .args(["-c", &command])
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Truncate to 4000 chars
            let stdout_trunc: String = stdout.chars().take(4000).collect();
            let stderr_trunc: String = stderr.chars().take(4000).collect();
            json!({
                "exit_code": output.status.code().unwrap_or(-1),
                "stdout": stdout_trunc,
                "stderr": stderr_trunc,
            })
        }
        Ok(Err(e)) => json!({ "error": format!("Command failed to execute: {}", e) }),
        Err(_) => json!({ "error": "Command timed out after 30 seconds" }),
    }
}

/// Delegate a task to Claude via the CLI.
async fn ask_claude(args: &Value) -> Value {
    let task = match args.get("task").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return json!({ "error": "Missing 'task' parameter" }),
    };

    log::info!("[Tool:ask_claude] Delegating: {}", task);

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::process::Command::new("claude")
            .args(["-p", &task, "--output-format", "text"])
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let response = String::from_utf8_lossy(&output.stdout);
            // Truncate to 8000 chars
            let response_trunc: String = response.chars().take(8000).collect();
            if output.status.success() {
                json!({ "response": response_trunc })
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                json!({ "error": format!("Claude exited with {}: {}", output.status, stderr.chars().take(2000).collect::<String>()) })
            }
        }
        Ok(Err(e)) => json!({ "error": format!("Failed to run Claude CLI: {}", e) }),
        Err(_) => json!({ "error": "Claude timed out after 120 seconds" }),
    }
}
