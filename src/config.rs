use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    Append,
    Replace,
    Delete,
}

#[derive(Debug, Deserialize)]
pub struct OperationConfig {
    pub file: PathBuf,
    pub heading: Vec<String>,
    #[serde(default)]
    pub index: usize,
    pub operation: OperationType,
    pub content: Option<String>,
    pub fingerprint: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    pub operations: Vec<OperationConfig>,
}

pub fn load_config(path: &PathBuf) -> Result<Vec<OperationConfig>> {
    let content = std::fs::read_to_string(path)?;
    let config: ConfigFile = serde_yaml::from_str(&content)?;
    
    // Validate operations
    for (i, op) in config.operations.iter().enumerate() {
        if op.heading.is_empty() {
            bail!("Operation {}: heading path cannot be empty", i + 1);
        }
        
        match op.operation {
            OperationType::Append | OperationType::Replace => {
                if op.content.is_none() {
                    bail!("Operation {}: content is required for append/replace", i + 1);
                }
            }
            OperationType::Delete => {}
        }
    }
    
    Ok(config.operations)
}
