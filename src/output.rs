use clap::ValueEnum;
use colored::Colorize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum OutputFormat {
    /// Unified diff format
    #[default]
    Diff,
    /// JSON format
    Json,
    /// Short summary
    Short,
}

#[derive(Serialize)]
struct JsonOutput {
    success: bool,
    applied: bool,
    changes: Vec<Change>,
}

#[derive(Serialize)]
struct Change {
    file: String,
    operation: String,
    heading: String,
    index: usize,
    status: String,
}

pub fn print_result(diff: &str, format: OutputFormat, applied: bool) {
    match format {
        OutputFormat::Diff => print_diff(diff),
        OutputFormat::Json => print_json(diff, applied),
        OutputFormat::Short => print_short(diff, applied),
    }
}

fn print_diff(diff: &str) {
    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            println!("{}", line.green());
        } else if line.starts_with('-') && !line.starts_with("---") {
            println!("{}", line.red());
        } else if line.starts_with("@") {
            println!("{}", line.cyan());
        } else {
            println!("{}", line);
        }
    }
}

fn print_json(_diff: &str, applied: bool) {
    // Simple JSON output - in production, this would be more structured
    let output = JsonOutput {
        success: true,
        applied,
        changes: vec![Change {
            file: "unknown".to_string(),
            operation: "patch".to_string(),
            heading: "unknown".to_string(),
            index: 0,
            status: if applied { "applied".to_string() } else { "dry-run".to_string() },
        }],
    };
    
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn print_short(diff: &str, applied: bool) {
    let additions = diff.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
    let deletions = diff.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
    
    let status = if applied {
        "Applied".green()
    } else {
        "Planned".yellow()
    };
    
    println!("{}: +{} -{}", status, additions, deletions);
}

pub fn format_diff(old: &str, new: &str, filename: &str) -> String {
    let mut diff = format!("--- a/{}\n+++ b/{}\n", filename, filename);
    
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    
    // Find approximate change location
    let mut start = 0;
    while start < old_lines.len() 
        && start < new_lines.len() 
        && old_lines[start] == new_lines[start] 
    {
        start += 1;
    }
    
    let mut old_end = old_lines.len();
    let mut new_end = new_lines.len();
    while old_end > start 
        && new_end > start 
        && old_lines[old_end - 1] == new_lines[new_end - 1] 
    {
        old_end -= 1;
        new_end -= 1;
    }
    
    // Output context and changes
    let context_start = start.saturating_sub(3);
    
    diff.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        context_start + 1,
        old_end.saturating_sub(context_start),
        context_start + 1,
        new_end.saturating_sub(context_start)
    ));
    
    // Context before
    for i in context_start..start {
        diff.push_str(&format!(" {}\n", old_lines[i]));
    }
    
    // Deletions
    for i in start..old_end {
        diff.push_str(&format!("-{}/n", old_lines[i]));
    }
    
    // Additions
    for i in start..new_end {
        diff.push_str(&format!("+{}\n", new_lines[i]));
    }
    
    // Context after
    for i in old_end..(old_end + 3).min(old_lines.len()) {
        diff.push_str(&format!(" {}\n", old_lines[i]));
    }
    
    diff
}
