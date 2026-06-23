//! 运行时环境识别模块
//!
//! 检测软件依赖的运行时环境（.NET、Java、Python、Go、Rust 等）。

use std::path::Path;

use packforge_core::RuntimeType;

/// 检测运行时环境
pub fn detect_runtime(exe_path: &Path, install_dir: &Path) -> RuntimeType {
    // 1. PE 导入表分析
    if let Ok(data) = std::fs::read(exe_path)
        && let Ok(pe) = goblin::pe::PE::parse(&data) {
            let imports: Vec<String> = pe
                .imports
                .iter()
                .map(|i| i.dll.to_lowercase())
                .collect();

            // .NET: 只导入 mscoree.dll（CLR 宿主）
            if imports.iter().any(|d| d == "mscoree.dll") {
                let version = detect_dotnet_version(install_dir);
                let self_contained = has_self_contained_dotnet(install_dir);
                return RuntimeType::DotNet {
                    version,
                    self_contained,
                };
            }

            // Go: PE 中含 "Go build ID"
            if crate::dependencies::contains_bytes_str(&data, "Go build ID") {
                return RuntimeType::Go;
            }

            // Rust: PE 中可能含 "rustc" 版本信息
            if crate::dependencies::contains_bytes_str(&data, "rustc") {
                return RuntimeType::Rust;
            }
        }

    // 2. 文件特征检测
    if install_dir.join("resources").join("app.asar").exists() {
        return RuntimeType::Node { is_electron: true };
    }

    if install_dir.join("jre").exists() || install_dir.join("jvm").exists() {
        return RuntimeType::Java {
            has_embedded_jre: true,
        };
    }

    if install_dir.join("python3.dll").exists()
        || install_dir.join("site-packages").exists()
    {
        return RuntimeType::Python {
            has_embedded_interpreter: true,
        };
    }

    RuntimeType::Native
}

/// 检测 .NET 版本
fn detect_dotnet_version(install_dir: &Path) -> String {
    // 尝试从 .runtimeconfig.json 读取
    for entry in walkdir::WalkDir::new(install_dir)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let name = entry.file_name().to_string_lossy();
        if name.ends_with(".runtimeconfig.json")
            && let Ok(content) = std::fs::read_to_string(entry.path())
                && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
                    && let Some(fw) = json
                        .get("runtimeOptions")
                        .and_then(|r| r.get("framework"))
                        .and_then(|f| f.get("version"))
                        .and_then(|v| v.as_str())
                    {
                        return fw.to_string();
                    }
    }

    "unknown".to_string()
}

/// 检测是否是 self-contained .NET 部署
fn has_self_contained_dotnet(install_dir: &Path) -> bool {
    install_dir.join("hostfxr.dll").exists()
        || install_dir.join("hostpolicy.dll").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_detected_for_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("app.exe");
        std::fs::write(&exe, b"MZ").unwrap(); // 简化
        // 简化的 PE 不会被 goblin 解析成功，应返回 Native
        let rt = detect_runtime(&exe, dir.path());
        assert_eq!(rt, RuntimeType::Native);
    }
}
