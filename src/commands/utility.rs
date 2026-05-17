use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::errors::BinanceError;
use crate::AppContext;

pub async fn run_shell(ctx: &AppContext) -> Result<(), BinanceError> {
    use colored::Colorize;

    println!("{}", "╔══════════════════════════════════════════╗".cyan());
    println!("{}", "║        Binance Interactive Shell          ║".cyan());
    println!("{}", "║  Type commands without 'binance' prefix  ║".cyan());
    println!("{}", "║  Type 'help' or 'exit' to quit            ║".cyan());
    println!("{}", "╚══════════════════════════════════════════╝".cyan());
    println!();

    let mut rl =
        DefaultEditor::new().map_err(|e| BinanceError::Io(std::io::Error::other(e.to_string())))?;

    let history_path = crate::config::Config::history_path()?;
    // Ensure parent directory exists for history file
    if let Some(parent) = history_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let _ = rl.load_history(&history_path);

    loop {
        let prompt = format!("{} ", "binance>".green().bold());
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(line);

                match line {
                    "exit" | "quit" | "q" => {
                        println!("{}", "Goodbye!".cyan());
                        break;
                    }
                    "help" | "h" | "?" => {
                        print_shell_help();
                        continue;
                    }
                    _ => {}
                }

                // Parse and execute as subcommand
                let parts = match shlex::split(line) {
                    Some(p) => p,
                    None => {
                        eprintln!("{} Invalid input", "Error:".red());
                        continue;
                    }
                };

                // Build full args: ["binance", ...parts]
                let mut args = vec!["binance".to_string()];
                args.extend(parts);

                // Try to parse and dispatch
                match crate::Cli::try_parse_from(&args) {
                    Ok(cli) => {
                        if matches!(cli.command, crate::Command::Shell) {
                            eprintln!("{}", "Error: nested shell is not supported".red());
                            continue;
                        }

                        let shell_ctx = AppContext {
                            client: ctx.client.clone(),
                            format: ctx.format,
                            verbose: cli.verbose || ctx.verbose,
                        };

                        match crate::dispatch_non_shell(&shell_ctx, cli.command).await {
                            Ok(out) => {
                                let rendered = out.render();
                                if !rendered.is_empty() {
                                    println!("{}", rendered);
                                }
                            }
                            Err(e) => {
                                crate::output::render_error(ctx.format, &e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("{}", "^C — use 'exit' to quit".yellow());
            }
            Err(ReadlineError::Eof) => {
                println!("{}", "Goodbye!".cyan());
                break;
            }
            Err(e) => {
                eprintln!("{} {}", "Readline error:".red(), e);
                break;
            }
        }
    }

    if rl.save_history(&history_path).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&history_path, std::fs::Permissions::from_mode(0o600));
        }
    }

    Ok(())
}

fn print_shell_help() {
    use colored::Colorize;
    println!("{}", "Available command groups:".bold());
    println!(
        "  {} — ping, server-time, price, orderbook, trades, klines, etc.",
        "market".cyan()
    );
    println!("  {} — info, balance, trades", "account".cyan());
    println!(
        "  {}   — buy, sell, cancel, query, open-orders, etc.",
        "trade".cyan()
    );
    println!(
        "  {} — withdraw, deposit-history, withdraw-history, deposit-address",
        "funding".cyan()
    );
    println!(
        "  {}      — depth, user, orders, balances (WebSocket streaming)",
        "ws".cyan()
    );
    println!(
        "  {}    — set, show, test, reset credentials",
        "auth".cyan()
    );
    println!("  {}   — balance simulation stub", "paper".cyan());
    println!();
    println!("  {}    — show this help", "help".yellow());
    println!("  {}    — quit the shell", "exit".yellow());
    println!();
    println!(
        "Example: {} {} {}",
        "market".cyan(),
        "price".white(),
        "BTCUSDT".green()
    );
}
