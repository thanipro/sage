use std::process::{Command, Stdio};
use std::path::Path;
use colored::Colorize;

use crate::error::{Result, SageError};

const MAX_DIFF_SIZE: usize = 15000;

/// Validate file paths to prevent command injection and ensure they're safe
fn validate_file_path(path: &str) -> Result<()> {
    // Check for null bytes (can cause issues with C APIs)
    if path.contains('\0') {
        return Err(SageError::InvalidInput(
            "File path contains null bytes".to_string()
        ));
    }

    // Check for suspicious patterns that might indicate command injection attempts
    let suspicious_patterns = [
        "|", "&", ";", "`", "$", "(", ")", "<", ">", "\n", "\r"
    ];

    for pattern in &suspicious_patterns {
        if path.contains(pattern) {
            return Err(SageError::InvalidInput(
                format!("File path contains suspicious character: {}", pattern)
            ));
        }
    }

    // Ensure path doesn't try to escape repository using absolute paths or ..
    let path_obj = Path::new(path);

    if path_obj.is_absolute() {
        return Err(SageError::InvalidInput(
            "Absolute paths are not allowed. Use relative paths within the repository".to_string()
        ));
    }

    for component in path_obj.components() {
        if let std::path::Component::ParentDir = component {
            return Err(SageError::InvalidInput(
                "Parent directory (..) traversal is not allowed".to_string()
            ));
        }
    }

    Ok(())
}

pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn get_diff(all: bool) -> Result<String> {
    let args = if all {
        vec!["diff"]
    } else {
        vec!["diff", "--cached"]
    };

    let output = Command::new("git")
        .args(args)
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::GitDiffFailed(error));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn get_files_changed(all: bool) -> Result<String> {
    let args = if all {
        vec!["status", "--porcelain"]
    } else {
        vec!["diff", "--cached", "--name-status"]
    };

    let output = Command::new("git")
        .args(args)
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::GitDiffFailed(error));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn stage_files(files: &[String]) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }

    // Validate all file paths before executing git command
    for file in files {
        validate_file_path(file)?;
    }

    let mut cmd = Command::new("git");
    cmd.arg("add");

    for file in files {
        cmd.arg(file);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::GitStagingFailed(error));
    }

    Ok(())
}

pub fn stage_all_files() -> Result<()> {
    let output = Command::new("git")
        .args(["add", "--all"])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::GitStagingFailed(error));
    }

    Ok(())
}

pub fn has_staged_changes() -> Result<bool> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .status()?;

    Ok(!output.success())
}

pub fn commit_changes(message: &str, amend: bool) -> Result<()> {
    let mut args = vec!["commit", "-m", message];

    if amend {
        args.push("--amend");
    }

    let output = Command::new("git")
        .args(args)
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::GitCommitFailed(error));
    }

    Ok(())
}

pub fn push_changes(force: bool) -> Result<()> {
    println!("{}", "Pushing changes...".blue());

    let mut args = vec!["push"];
    if force {
        args.push("--force");
    }

    let output = Command::new("git")
        .args(args)
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::GitPushFailed(error));
    }

    println!("{}", "Changes pushed successfully!".green());
    Ok(())
}

pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::GitDiffFailed(error));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn create_and_checkout_branch(branch_name: &str) -> Result<()> {
    // Create the branch
    let output = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(SageError::InvalidInput(
            format!("Failed to create branch '{}': {}", branch_name, error)
        ));
    }

    println!("{}", format!("Switched to new branch '{}'", branch_name).green());
    Ok(())
}

pub fn branch_exists(branch_name: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", branch_name])
        .output()?;

    Ok(output.status.success())
}

pub fn show_changes(diff: &str, files_changed: &str) -> Result<()> {
    println!("{}", "Changes to be committed:".blue().bold());
    println!("{}", files_changed);
    println!("\n{}", "Diff:".blue().bold());

    let display_len = std::cmp::min(diff.len(), 2000);
    if display_len > 0 {
        println!("{}", &diff[..display_len]);
        if diff.len() > 2000 {
            println!("... [truncated for display]");
        }
    } else {
        println!("(empty diff)");
    }

    Ok(())
}

pub fn smart_truncate_diff(diff: &str) -> String {
    if diff.len() <= MAX_DIFF_SIZE {
        return diff.to_string();
    }

    let mut important_parts = String::new();

    for chunk in diff.split("diff --git").skip(1) {
        let chunk_lines: Vec<&str> = chunk.lines().collect();
        let header_lines = std::cmp::min(5, chunk_lines.len());

        important_parts.push_str("diff --git");
        for i in 0..header_lines {
            if i < chunk_lines.len() {
                important_parts.push_str(chunk_lines[i]);
                important_parts.push('\n');
            }
        }

        if chunk_lines.len() > 20 {
            important_parts.push_str("...[truncated]...\n");
            let mid = chunk_lines.len() / 2;
            for i in mid..(mid + 3).min(chunk_lines.len()) {
                important_parts.push_str(chunk_lines[i]);
                important_parts.push('\n');
            }
        }

        if important_parts.len() > MAX_DIFF_SIZE / 2 {
            break;
        }
    }

    if important_parts.len() < MAX_DIFF_SIZE {
        important_parts
    } else {
        format!("{}... [truncated - diff too large]", &diff[0..MAX_DIFF_SIZE / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_truncate_diff() {
        let small_diff = "diff --git a/file.txt b/file.txt\nindex 123..456 789\n--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,3 @@\n-old line\n+new line\n context";
        assert_eq!(smart_truncate_diff(small_diff), small_diff);

        let large_diff = "a".repeat(MAX_DIFF_SIZE + 1000);
        assert!(smart_truncate_diff(&large_diff).len() <= MAX_DIFF_SIZE);
    }
}
