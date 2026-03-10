mod models;
mod storage;
mod commands;

use std::env;
use std::error::Error;

fn print_help() {
    println!("Snippet Manager v0.0.1");
    println!();
    println!("Commands:");
    println!("  add               Add a new snippet");
    println!("  list              List all snippets");
    println!("  search <query>    Search snippets");
    println!("  view <id>         View snippet by id");
    println!("  delete <id>       Delete snippet by id");
    println!("  help              Show this help");
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "add" => commands::add_snippet()?,
        "list" => commands::list_snippets()?,
        "search" => {
            if args.len() < 3 {
                println!("Please provide a search query.");
            } else {
                let query = args[2..].join(" ");
                commands::search_snippets(&query)?;
            }
        }
        "view" => {
            if args.len() < 3 {
                println!("Please provide a snippet id.");
            } else {
                commands::view_snippet(&args[2])?;
            }
        }
        "delete" => {
            if args.len() < 3 {
                println!("Please provide a snippet id.");
            } else {
                commands::delete_snippet(&args[2])?;
            }
        }
        "help" => print_help(),
        _ => {
            println!("Unknown command.");
            print_help();
        }
    }

    Ok(())
}