//! 关键特性回归测试
//! 
//! 运行: cargo test --test integration_test

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// 获取 mdp 二进制路径
fn mdp_bin() -> PathBuf {
    // 优先使用当前构建的二进制（通过 CARGO_BIN_EXE_mdp 环境变量）
    if let Ok(bin_path) = std::env::var("CARGO_BIN_EXE_mdp") {
        return PathBuf::from(bin_path);
    }
    
    // 否则尝试 release 或 debug 版本
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("mdp");
    
    if !path.exists() {
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("target");
        path.push("debug");
        path.push("mdp");
    }
    
    path
}

/// 创建临时 markdown 文件（使用线程安全的唯一名称）
fn create_test_file(content: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();
    
    let temp_dir = std::env::temp_dir();
    let file_name = format!("mdp_test_{:?}_{}_{}.md", thread_id, timestamp, counter);
    let file_path = temp_dir.join(file_name);
    fs::write(&file_path, content).unwrap();
    file_path
}

/// 运行 mdp 命令，返回 (exit_code, stdout, stderr)
fn run_mdp(args: &[&str]) -> (i32, String, String) {
    let bin = mdp_bin();
    let output = Command::new(&bin)
        .args(args)
        .output()
        .expect(&format!("Failed to execute {:?}", bin));
    
    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    (exit_code, stdout, stderr)
}

// ============================================================================
// 测试：幂等性 (关键特性)
// ============================================================================

#[test]
fn test_idempotent_append() {
    let content = "# Doc\n\n## UniqueSection\n\nOriginal\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    // 第一次 append
    let (code1, _, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## UniqueSection",
        "--op", "append",
        "-c", "New content",
        "--force"
    ]);
    assert_eq!(code1, 0, "First append should succeed");
    
    let content_after_first = fs::read_to_string(&file_path).unwrap();
    assert!(content_after_first.contains("New content"), "Content should be added");
    
    // 第二次 append相同内容（幂等性测试）
    let (code2, stdout2, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## UniqueSection",
        "--op", "append",
        "-c", "New content",
        "--force"
    ]);
    assert_eq!(code2, 0, "Second append should succeed");
    
    // 检查 diff 中没有新增内容（说明幂等性生效）
    let content_after_second = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content_after_first, content_after_second, "Should be idempotent - no changes on second run");
    
    // 清理
    let _ = fs::remove_file(&file_path);
}

// ============================================================================
// 测试：安全机制 (关键特性)
// ============================================================================

#[test]
fn test_destructive_operation_requires_fingerprint_or_force() {
    let content = "# Doc\n\n## UniqueSensitive\n\nSensitive content\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    // 不带 fingerprint 和 --force 的 replace 应该失败
    let (code, _, stderr) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## UniqueSensitive",
        "--op", "replace",
        "-c", "New content"
    ]);
    
    assert_ne!(code, 0, "Should fail without fingerprint or force");
    assert!(stderr.contains("fingerprint") || stderr.contains("force") || stderr.contains("safety"), 
            "Error should mention fingerprint, force, or safety: {}", stderr);
    
    // 清理
    let _ = fs::remove_file(&file_path);
}

#[test]
fn test_fingerprint_validation() {
    let content = "# Doc\n\n## TodoSection\n\nTODO: fix this\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    // 错误的 fingerprint
    let (code, _, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## TodoSection",
        "--op", "replace",
        "-c", "Fixed",
        "-p", "WRONG_PATTERN",
        "--force"
    ]);
    assert_eq!(code, 3, "Should exit with code 3 for fingerprint mismatch");
    
    // 正确的 fingerprint
    let (code2, _, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## TodoSection",
        "--op", "replace",
        "-c", "Fixed",
        "-p", "TODO.*fix",
        "--force"
    ]);
    assert_eq!(code2, 0, "Should succeed with correct fingerprint");
    
    let result = fs::read_to_string(&file_path).unwrap();
    assert!(result.contains("Fixed"), "Content should be replaced");
    
    // 清理
    let _ = fs::remove_file(&file_path);
}

// ============================================================================
// 测试：退出码 (关键特性)
// ============================================================================

#[test]
fn test_exit_code_heading_not_found() {
    let content = "# Doc\n\nContent\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    let (code, _, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## NonExistent",
        "--op", "append",
        "-c", "x"
    ]);
    
    assert_eq!(code, 2, "Should exit with code 2 for heading not found");
    
    // 清理
    let _ = fs::remove_file(&file_path);
}

#[test]
fn test_exit_code_ambiguous_heading() {
    let content = "# Doc A\n\n## AmbigSection\n\nA\n\n# Doc B\n\n## AmbigSection\n\nB\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    let (code, _, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## AmbigSection",
        "--op", "append",
        "-c", "x"
    ]);
    
    assert_eq!(code, 4, "Should exit with code 4 for ambiguous heading");
    
    // 清理
    let _ = fs::remove_file(&file_path);
}

// ============================================================================
// 测试：嵌套 Heading 路径 (关键特性)
// ============================================================================

#[test]
fn test_nested_heading_path() {
    let content = "# Doc A\n\n## Section\n\nContent A\n\n# Doc B\n\n## Section\n\nContent B\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    // 使用完整路径指定第一个 Section
    let (code, _, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "# Doc A ## Section",
        "--op", "append",
        "-c", "Added to A",
        "--force"
    ]);
    
    assert_eq!(code, 0, "Should succeed with nested path");
    
    let result = fs::read_to_string(&file_path).unwrap();
    // 检查内容被添加到正确的位置（Doc A 下）
    let pos_a = result.find("Content A").unwrap();
    let pos_b = result.find("Content B").unwrap();
    let pos_added = result.find("Added to A").unwrap();
    
    assert!(pos_added > pos_a && pos_added < pos_b, 
            "Content should be added between A and B");
    
    // 清理
    let _ = fs::remove_file(&file_path);
}

// ============================================================================
// 测试：原子操作
// ============================================================================

#[test]
fn test_atomic_replace() {
    let content = "# Doc\n\n## AtomicSection\n\nOriginal\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    // 执行 replace
    let (code, _, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## AtomicSection",
        "--op", "replace",
        "-c", "Replaced",
        "-p", "Original",
        "--force"
    ]);
    
    assert_eq!(code, 0);
    
    let result = fs::read_to_string(&file_path).unwrap();
    assert!(result.contains("Replaced"));
    assert!(!result.contains("Original"));
    
    // 检查没有遗留的临时文件
    let temp_file = file_path.with_extension("md.tmp");
    assert!(!temp_file.exists(), "Temp file should be cleaned up");
    
    // 清理
    let _ = fs::remove_file(&file_path);
}

// ============================================================================
// 测试：JSON 输出
// ============================================================================

#[test]
fn test_json_output() {
    let content = "# Doc\n\n## JsonSection\n\nContent\n";
    let file_path = create_test_file(content);
    let file_str = file_path.to_str().unwrap();
    
    let (code, stdout, _) = run_mdp(&[
        "patch",
        "-f", file_str,
        "-H", "## JsonSection",
        "--op", "append",
        "-c", "New",
        "--force",
        "-F", "json"
    ]);
    
    assert_eq!(code, 0);
    assert!(stdout.contains("\"file\""), "JSON should contain 'file' field");
    assert!(stdout.contains("\"operation\""), "JSON should contain 'operation' field");
    assert!(stdout.contains("\"heading\""), "JSON should contain 'heading' field");
    
    // 验证不是硬编码的 "unknown"
    assert!(!stdout.contains("\"unknown\""), "JSON fields should have real values, not 'unknown'");
    
    // 清理
    let _ = fs::remove_file(&file_path);
}
