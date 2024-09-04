use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

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
    #[serde(rename = "type")]
    pub r#type: String,
    pub sha: String,
    pub size: Option<u64>,
    pub url: Option<String>,
}

#[derive(Debug)]
struct LanguageStats {
    files: usize,
}

impl LanguageStats {
    fn new() -> Self {
        Self { files: 0 }
    }
}

pub async fn fetch_and_display_tree(github_url: &str) -> Result<(), Box<dyn Error>> {
    let (owner, repo) = extract_owner_repo(github_url)?;
    let client = reqwest::Client::new();

    // Fetch repository info
    let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
    let repo_res = client
        .get(&repo_url)
        .header(USER_AGENT, "rust-tool")
        .send()
        .await?;

    if !repo_res.status().is_success() {
        eprintln!(
            "Failed to fetch repository info: {} - {}",
            repo_res.status(),
            repo_res.text().await?
        );
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
    let tree_res = client
        .get(&tree_url)
        .header(USER_AGENT, "rust-tool")
        .send()
        .await?;

    if tree_res.status().is_success() {
        let tree: GitTree = tree_res.json().await?;
        crate::display::print_tree(&tree.tree, 0);

        // Fetch file contents
        let files = fetch_files(&client, &tree.tree).await?;
        let (language_stats, framework_message) = analyze_files(&files);
        display_language_stats(&language_stats, framework_message);

    } else {
        eprintln!(
            "Failed to fetch the repo tree: {} - {}",
            tree_res.status(),
            tree_res.text().await?
        );
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

async fn fetch_files(
    client: &reqwest::Client,
    tree: &[TreeNode],
) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut files = HashMap::new();

    for node in tree {
        if node.r#type == "blob" {
            let url = match &node.url {
                Some(url) => url,
                None => {
                    eprintln!("Skipping file {} due to missing URL.", node.path);
                    continue;
                }
            };

            let file_res = client.get(url).header(USER_AGENT, "rust-tool").send().await?;

            if file_res.status().is_success() {
                let content = file_res.text().await?;
                files.insert(node.path.clone(), content);
            } else {
                eprintln!(
                    "Failed to fetch file {}: {} - {}",
                    node.path,
                    file_res.status(),
                    file_res.text().await?
                );
            }
        }
    }

    Ok(files)
}


fn detect_language(path: &str) -> String {
    let mut languages = HashMap::new();
    languages.insert(".rs", "Rust");
    languages.insert(".js", "JavaScript");
    languages.insert(".mjs", "JavaScript");
    languages.insert(".ts", "TypeScript");
    languages.insert(".css", "CSS");
    languages.insert(".html", "HTML");

    for (ext, lang) in &languages {
        if path.ends_with(ext) {
            return lang.to_string();
        }
    }

    if path.contains("<!DOCTYPE html>") {
        return "HTML".to_string();
    }

    "Unknown".to_string()
}

fn detect_framework(path: &str, content: &str) -> String {
    let mut frameworks = HashMap::new();
    frameworks.insert("next.config.js", "Next.js");
    frameworks.insert("next.config.mjs", "Next.js");
    frameworks.insert("vue.config", "Vue.js");
    frameworks.insert("angular.json", "Angular");

    // Detect based on path contents
    for (key, framework) in &frameworks {
        if path.contains(key) {
            return framework.to_string();
        }
    }

    // Additional detection for frameworks in package.json
    let mut package_json_frameworks = HashMap::new();
    package_json_frameworks.insert("react", "React");
    package_json_frameworks.insert("vue", "Vue.js");
    package_json_frameworks.insert("angular", "Angular");

    if path.contains("package.json") {
        for (key, framework) in &package_json_frameworks {
            if content.contains(key) {
                return framework.to_string();
            }
        }
    }

    "None".to_string()
}
fn analyze_files(files: &HashMap<String, String>) -> (HashMap<String, LanguageStats>, Option<String>) {
    let mut language_stats = HashMap::new();
    let mut framework = None;
    let mut has_website_files = false;

    for (path, content) in files {
        let language = detect_language(path);
        let current_framework = detect_framework(path, content);

        if current_framework != "None" {
            framework = Some(current_framework);
        }

        // Check if the file indicates a website
        if path.ends_with(".html") || path.ends_with(".css") {
            has_website_files = true;
        }

        let lang_entry = language_stats.entry(language).or_insert_with(LanguageStats::new);
        lang_entry.files += 1;
    }

    let framework_message = if has_website_files {
        if let Some(framework) = framework {
            format!("This project is a website made with {}", framework)
        } else {
            "This project is a static website".to_string()
        }
    } else {
        "No website-related files detected".to_string()
    };

    (language_stats, Some(framework_message))
}

fn display_language_stats(language_stats: &HashMap<String, LanguageStats>, framework_message: Option<String>) {
    println!("Languages Used:");
    println!("===============================================================================");
    
    for (language, stats) in language_stats {
        println!("Language: {}", language);
        println!("Files: {}", stats.files);
        println!("--------------------------------------------------");
    }

    println!("===============================================================================");
    
    if let Some(message) = framework_message {
        println!("{}", message);
    }
}
