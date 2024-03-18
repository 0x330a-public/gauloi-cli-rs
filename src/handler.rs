use std::collections::HashMap;

use async_trait::async_trait;
use shellfish::{AsyncHandler, Command};
use shellfish::command::CommandType;
use yansi::Paint;

#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub struct GauloiAsyncHandler;

#[async_trait]
impl<T: Send> AsyncHandler<T> for GauloiAsyncHandler {
    async fn handle_async(
        &self,
        line: Vec<String>,
        commands: &HashMap<&str, Command<T>>,
        state: &mut T,
        description: &str,
    ) -> bool {
        if let Some(command) = line.get(0) {
            let command_str = command.as_str();
            match command_str {
                "quit" | "q" => return true,
                "help" | "h" => {
                    println!("{}", description);

                    // Print default and commands
                    println!("    help | h - displays this help information.");
                    println!("    quit | q - quits the shell.");
                    println!();
                    for (name, command) in commands {
                        println!("    {} - {}", name, command.help);
                    }
                }
                _ => {
                    let command = commands.get(command_str);
                    match command {
                        Some(command) => {
                            if let Err(e) = match command.command {
                                CommandType::Sync(c) => c(state, line),
                                CommandType::Async(a) => a(state, line).await,
                            } {
                                eprintln!(
                                    "{}",
                                    Paint::red(format!(
                                        "Command exited unsuccessfully:\n{}\n({:?})",
                                        &e, &e
                                    ))
                                )
                            }
                        }
                        None => {
                            eprintln!("{} {}", Paint::red("Command not found:"), command_str)
                        }
                    }
                }
            }
            println!() // Padding
        }
        false
    }
}
