//! Simple REPL: reads SQL from stdin, executes it against a
//! persistent in-memory `Database`, and prints the result. Buffers
//! input across lines until a `;`-terminated statement is seen, so
//! multi-line statements work. Enter `.exit`, `.quit`, or Ctrl-D to
//! quit.

use std::io::{self, BufRead, Write};

use basaltsql::db::Database;
use basaltsql::executor::ExecResult;
use basaltsql::types::Value;

fn main() {
    println!("basaltsql v{}", basaltsql::VERSION);

    let mut db = Database::new();
    let stdin = io::stdin();
    let mut buffer = String::new();

    print_banner();
    prompt(&buffer);

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                eprintln!("error reading input: {err}");
                break;
            }
        };

        if buffer.is_empty() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                prompt(&buffer);
                continue;
            }
            if matches!(trimmed, ".exit" | ".quit" | "exit" | "quit") {
                break;
            }
        }

        buffer.push_str(&line);
        buffer.push('\n');

        if buffer.trim_end().ends_with(';') {
            match db.execute_all(&buffer) {
                Ok(results) => results.iter().for_each(print_result),
                Err(err) => println!("Error: {err}"),
            }
            buffer.clear();
        }

        prompt(&buffer);
    }
    println!();
}

fn print_banner() {
    println!("BasaltSQL — type SQL statements ending in ';', or .exit to quit.");
}

fn prompt(buffer: &str) {
    print!(
        "{}",
        if buffer.is_empty() {
            "basalt> "
        } else {
            "   ...> "
        }
    );
    io::stdout().flush().ok();
}

fn print_result(result: &ExecResult) {
    match result {
        ExecResult::Rows(rows) => {
            if rows.is_empty() {
                println!("(0 rows)");
            } else {
                for row in rows {
                    let rendered: Vec<String> = row.values.iter().map(format_value).collect();
                    println!("{}", rendered.join(" | "));
                }
                println!(
                    "({} row{})",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                );
            }
        }
        ExecResult::RowsAffected(n) => {
            println!("{n} row{} affected", if *n == 1 { "" } else { "s" });
        }
        ExecResult::TableCreated => println!("Table created."),
    }
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Integer(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Text(s) => s.clone(),
        Value::Boolean(b) => b.to_string(),
        Value::Null => "NULL".to_string(),
    }
}
