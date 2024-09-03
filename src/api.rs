use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::error::Error;
use std::process::Command;
use std::str;

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
    let readme_url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/README.md",
        owner, repo, default_branch
    );

    println!("Tree URL: {}", tree_url);
    println!("README.md URL: {}", readme_url);

    // Fetch tree
    let tree_res = client.get(&tree_url)
        .header(USER_AGENT, "rust-tool")
        .send()
        .await?;
    
    if tree_res.status().is_success() {
        let tree: GitTree = tree_res.json().await?;
        crate::display::print_tree(&tree.tree, 0);

        // Clone repository and run Tokei
        clone_repo(&owner, &repo).await?;
        let languages = analyze_with_tokei().await?;
        println!("Languages Used:\n{}", languages);
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

async fn clone_repo(owner: &str, repo: &str) -> Result<(), Box<dyn Error>> {
    let repo_url = format!("https://github.com/{}/{}.git", owner, repo);
    let output = Command::new("git")
        .args(&["clone", &repo_url])
        .output()?;

    if !output.status.success() {
        eprintln!("Failed to clone repository: {}", str::from_utf8(&output.stderr)?);
    }
    Ok(())
}

async fn analyze_with_tokei() -> Result<String, Box<dyn Error>> {
    let output = Command::new("tokei")
        .arg(".")
        .output()?;

    if !output.status.success() {
        eprintln!("Failed to run Tokei: {}", str::from_utf8(&output.stderr)?);
        return Ok("Failed to run Tokei".to_string());
    }

    let output_str = str::from_utf8(&output.stdout)?;
    Ok(output_str.to_string())
}
