use project_type_checker::api::fetch_and_display_tree; // Correct module path

#[tokio::main]
async fn main() {
    use std::io::{self, Write};
    let mut input = String::new();

    loop {
        print!("Enter the GitHub repository URL (e.g., https://github.com/owner/repo) or type 'exit' to quit: ");
        io::stdout().flush().expect("Failed to flush stdout");
        input.clear();
        io::stdin().read_line(&mut input).expect("Failed to read line");
        let url = input.trim();

        if url == "exit" {
            break;
        }

        if let Err(err) = fetch_and_display_tree(url).await {
            eprintln!("Error: {}", err);
        }
    }
}