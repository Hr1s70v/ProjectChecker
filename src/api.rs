use reqwest::header::USER_AGENT;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::env;
use std::path::Path;

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
struct FileStats {
    files: usize,
}

impl FileStats {
    fn new() -> Self {
        Self { files: 0 }
    }
}

#[derive(Deserialize)]
struct FileTypes {
    programming_languages: HashMap<String, Vec<String>>,
    web_files: HashMap<String, Vec<String>>,
    config_files: HashMap<String, Vec<String>>,
    documentation: HashMap<String, Vec<String>>,
    images: HashMap<String, Vec<String>>,
    video: HashMap<String, Vec<String>>,
    audio: HashMap<String, Vec<String>>,
    archives: HashMap<String, Vec<String>>,
    fonts: HashMap<String, Vec<String>>,
    other: HashMap<String, Vec<String>>,
}

#[derive(Deserialize)]
pub struct FileMappings {
    file_types: FileTypes,
}

pub fn load_file_mappings() -> Result<FileMappings, Box<dyn std::error::Error>> {
    // Dynamically construct the path to 'src/extensions.json'
    let _current_dir = env::current_dir()?;
    let path = Path::new("./extensions.json");
    if !path.exists() {
        println!("File does not exist at path: {:?}", path.display());
    }

    // Check if the file exists
    if !path.exists() {
        return Err(format!("File not found: {}", path.display()).into());
    }

    // Try opening the file using the dynamically constructed path
    let file = File::open(&path)
        .map_err(|e| format!("Failed to open file '{}': {}", path.display(), e))?;

    let mappings: FileMappings = serde_json::from_reader(file)
        .map_err(|e| format!("Failed to parse JSON from '{}': {}", path.display(), e))?;

    Ok(mappings)
}

async fn detect_file_type(path: &str, mappings: &FileMappings) -> String {
    let all_types = vec![
        &mappings.file_types.programming_languages,
        &mappings.file_types.web_files,
        &mappings.file_types.config_files,
        &mappings.file_types.documentation,
        &mappings.file_types.images,
        &mappings.file_types.video,
        &mappings.file_types.audio,
        &mappings.file_types.archives,
        &mappings.file_types.fonts,
        &mappings.file_types.other,
    ];

    for types_map in all_types {
        for (file_type, patterns) in types_map {
            for pattern in patterns {
                if path.ends_with(pattern.trim_start_matches('*')) {
                    println!("Matched file type: {} for file: {}", file_type, path);
                    return file_type.clone();
                }
            }
        }
    }

    println!("Unknown file type for file: {}", path);
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

async fn analyze_files(
    files: &HashMap<String, String>,
    mappings: &FileMappings,
) -> (HashMap<String, FileStats>, Option<String>) {
    let mut file_stats = HashMap::new();
    let mut framework = None;
    let mut has_website_files = false;

    for (path, content) in files {
        let file_type = detect_file_type(path, mappings).await;
        let current_framework = detect_framework(path, content);

        if current_framework != "None" {
            framework = Some(current_framework);
        }

        // Check if the file indicates a website
        if path.ends_with(".html") || path.ends_with(".css") {
            has_website_files = true;
        }

        let type_entry = file_stats.entry(file_type).or_insert_with(FileStats::new);
        type_entry.files += 1;
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

    (file_stats, Some(framework_message))
}

fn display_file_stats(file_stats: &HashMap<String, FileStats>, framework_message: Option<String>) {
    println!("Repository contents:");
    println!("--------------------------------------------------");
    
    for (file_type, stats) in file_stats {
        println!("File Type: {}", file_type);
        println!("Files: {}", stats.files);
        println!("--------------------------------------------------");
    }
    
    if let Some(message) = framework_message {
        println!("{}", message);
    }
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

pub async fn fetch_and_display_tree(github_url: &str) -> Result<(), Box<dyn Error>> {
    let (owner, repo) = extract_owner_repo(github_url)?;
    let client = reqwest::Client::new();

    // Load file mappings
    let mappings = match load_file_mappings() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error loading file mappings: {}", e);
            return Ok(());
        }
    };

    // Fetch repository info
    let repo_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
    let repo_res = client
        .get(&repo_url)
        .header(USER_AGENT, "rust-tool")
        .send().await?;

    if !repo_res.status().is_success() {
        eprintln!(
            "Failed to fetch repository info: {} - {}",
            repo_res.status(),
            repo_res.text().await?
        );
        return Ok(()); // or Err(e) if you want to propagate the error
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
        .send().await?;

    if tree_res.status().is_success() {
        let tree: GitTree = tree_res.json().await?;
        crate::display::print_tree(&tree.tree, 0);

        // Fetch file contents
        let files = fetch_files(&client, &tree.tree).await?;
        let (file_stats, framework_message) = analyze_files(&files, &mappings).await;
        display_file_stats(&file_stats, framework_message);

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
    let url_parts: Vec<&str> = github_url.split('/').collect();
    if url_parts.len() < 5 {
        return Err(format!("Invalid GitHub URL: {}", github_url).into());
    }
    let owner = url_parts[3].to_string();
    let repo = url_parts[4].to_string();
    Ok((owner, repo))
}

pub fn print_tree(tree: &[TreeNode], level: usize) {
    for node in tree {
        for _ in 0..level {
            print!("  ");
        }
        println!("{}", node.path);
        if node.r#type == "tree" {
            // If it's a directory, recursively print its contents
            if let Some(url) = &node.url {
                if let Ok(sub_tree) = fetch_sub_tree(url) {
                    print_tree(&sub_tree.tree, level + 1);
                }
            }
        }
    }
}

fn fetch_sub_tree(url: &str) -> Result<GitTree, Box<dyn Error>> {
    let client = reqwest::blocking::Client::new();
    let tree_res = client.get(url).header(USER_AGENT, "rust-tool").send()?;

    if tree_res.status().is_success() {
        let tree: GitTree = tree_res.json()?;
        Ok(tree)
    } else {
        eprintln!(
            "Failed to fetch the sub-tree: {} - {}",
            tree_res.status(),
            tree_res.text()?
        );
        Err("Failed to fetch the sub-tree".into())
    }
}