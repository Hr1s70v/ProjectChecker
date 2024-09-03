use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::error::Error;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
pub struct GitTree {
    pub sha: String,
    pub url: String,
    pub tree: Vec<TreeNode>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TreeNode {
    pub path: String,
    pub mode: String,
    pub r#type: String,
    pub sha: String,
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug)]
struct LanguageStats {
    files: usize,
    lines: usize,
    code: usize,
    comments: usize,
    blanks: usize,
}

impl LanguageStats {
    fn new() -> Self {
        Self {
            files: 0,
            lines: 0,
            code: 0,
            comments: 0,
            blanks: 0,
        }
    }
}

pub async fn fetch_and_display_tree(github_url: &str) -> Result<(), Box<dyn Error>> {
    let (owner, repo) = extract_owner_repo(github_url)?;
    let client = reqwest::Client::new();

    // Fetch repository info
    let repo_url = format!(
        "https://api.github.com/repos/{}/{}",
        owner, repo
    );
    let repo_res = client.get(&repo_url)
        .header(USER_AGENT, "rust-tool")
        .send()
        .await?;

    if !repo_res.status().is_success() {
        eprintln!("Failed to fetch repository info: {} - {}", repo_res.status(), repo_res.text().await?);
        return Ok(());
    }

    let repo_info: serde_json::Value = repo_res.json().await?;
    let default_branch = repo_info["default_branch"]
        .as_str()
        .unwrap_or("main")
        .to_string();

    let tree_url = format!(
        "https://api.github.com/repos/{}/{}/git/trees/{}?recursive=1",
        owner, repo, default_branch
    );

    println!("Tree URL: {}", tree_url);

    // Fetch tree
    let tree_res = client.get(&tree_url)
        .header(USER_AGENT, "rust-tool")
        .send()
        .await?;
    
    if tree_res.status().is_success() {
        let tree: GitTree = tree_res.json().await?;
        crate::display::print_tree(&tree.tree, 0);

        // Fetch file contents
        let files = fetch_files(&client, &tree.tree).await?;
        let languages = analyze_files(&files);
        display_language_stats(&languages);
    } else {
        eprintln!("Failed to fetch the repo tree: {} - {}", tree_res.status(), tree_res.text().await?);
    }

    Ok(())
}

fn extract_owner_repo(github_url: &str) -> Result<(String, String), Box<dyn Error>> {
    let url_parts: Vec<&str> = github_url.trim_end_matches('/').split('/').collect();
    if url_parts.len() < 2 {
        return Err("Invalid GitHub URL format.".into());
    }
    let owner = url_parts[url_parts.len() - 2].to_string();
    let repo = url_parts[url_parts.len() - 1].to_string();
    Ok((owner, repo))
}

async fn fetch_files(client: &reqwest::Client, tree: &[TreeNode]) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut files = HashMap::new();

    for node in tree {
        if node.r#type == "blob" {
            let file_res = client.get(&node.url)
                .header(USER_AGENT, "rust-tool")
                .send()
                .await?;

            if file_res.status().is_success() {
                let content = file_res.text().await?;
                files.insert(node.path.clone(), content);
            } else {
                eprintln!("Failed to fetch file {}: {} - {}", node.path, file_res.status(), file_res.text().await?);
            }
        }
    }

    Ok(files)
}

fn analyze_files(files: &HashMap<String, String>) -> HashMap<String, LanguageStats> {
    let mut language_stats = HashMap::new();

    for (path, content) in files {
        let language = detect_language(path, content);
        let stats = count_lines(content);

        let entry = language_stats.entry(language).or_insert_with(LanguageStats::new);
        entry.files += 1;
        entry.lines += stats.lines;
        entry.code += stats.code;
        entry.comments += stats.comments;
        entry.blanks += stats.blanks;
    }

    language_stats
}

fn detect_language(path: &str, content: &str) -> String {
    // Detect configuration files
    if path.contains("next.config") {
        return "Next.js".to_string();
    } else if path.contains("vue.config") {
        return "Vue.js".to_string();
    } else if path.contains("angular.json") {
        return "Angular".to_string();
    } else if path.contains("package.json") && content.contains("\"react\"") {
        return "React".to_string();
    }

    // Basic language detection based on file extension
    if path.ends_with(".rs") {
        "Rust".to_string()
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "JavaScript".to_string()
    } else if path.ends_with(".ts") {
        "TypeScript".to_string()
    } else if path.ends_with(".css") {
        "CSS".to_string()
    } else if path.ends_with(".html") || content.contains("<!DOCTYPE html>") {
        "HTML".to_string()
    } else {
        "Unknown".to_string()
    }
}

fn count_lines(content: &str) -> LanguageStats {
    let mut stats = LanguageStats::new();
    for line in content.lines() {
        stats.lines += 1;
        if line.trim().is_empty() {
            stats.blanks += 1;
        } else if line.trim().starts_with("//") || line.trim().starts_with("/*") {
            stats.comments += 1;
        } else {
            stats.code += 1;
        }
    }
    stats
}

fn display_language_stats(language_stats: &HashMap<String, LanguageStats>) {
    println!("Languages Used:");
    println!("===============================================================================");
    println!(" Language            Files        Lines         Code     Comments       Blanks");
    println!("===============================================================================");

    for (language, stats) in language_stats {
        println!("{:<20} {:<12} {:<12} {:<12} {:<12} {:<12}", 
            language, stats.files, stats.lines, stats.code, stats.comments, stats.blanks);
    }

    println!("===============================================================================");
    let total_files: usize = language_stats.values().map(|s| s.files).sum();
    let total_lines: usize = language_stats.values().map(|s| s.lines).sum();
    let total_code: usize = language_stats.values().map(|s| s.code).sum();
    let total_comments: usize = language_stats.values().map(|s| s.comments).sum();
    let total_blanks: usize = language_stats.values().map(|s| s.blanks).sum();
    println!(" Total               {:<12} {:<12} {:<12} {:<12} {:<12}", 
        total_files, total_lines, total_code, total_comments, total_blanks);
    println!("===============================================================================");
}
