//! 依赖分析模块
//!
//! 分析 PE 文件的导入表，追踪 DLL 依赖链。

use std::collections::{HashSet, VecDeque};
use std::path::Path;


use crate::Result;

/// 已知的 Windows 系统 DLL（不应包含在便携包中）
const SYSTEM_DLLS: &[&str] = &[
    "kernel32.dll",
    "kernelbase.dll",
    "ntdll.dll",
    "user32.dll",
    "gdi32.dll",
    "gdi32full.dll",
    "advapi32.dll",
    "advapi32res.dll",
    "shell32.dll",
    "shlwapi.dll",
    "shcore.dll",
    "ole32.dll",
    "oleaut32.dll",
    "comctl32.dll",
    "comdlg32.dll",
    "msvcrt.dll",
    "ucrtbase.dll",
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "ws2_32.dll",
    "wsock32.dll",
    "winmm.dll",
    "winspool.drv",
    "crypt32.dll",
    "bcrypt.dll",
    "ncrypt.dll",
    "rpcrt4.dll",
    "secur32.dll",
    "iphlpapi.dll",
    "dnsapi.dll",
    "setupapi.dll",
    "cfgmgr32.dll",
    "powrprof.dll",
    "profapi.dll",
    "version.dll",
    "imm32.dll",
    "msimg32.dll",
    "dwmapi.dll",
    "uxtheme.dll",
    "propsys.dll",
];

/// 依赖树节点
#[derive(Debug, Clone)]
pub struct DepTreeNode {
    pub path: std::path::PathBuf,
    pub dll_name: String,
    pub is_system: bool,
    pub depth: usize,
}

/// 依赖树
#[derive(Debug)]
pub struct DepTree {
    pub nodes: Vec<DepTreeNode>,
    pub edges: Vec<(usize, usize)>, // (from_idx, to_idx)
}

impl Default for DepTree {
    fn default() -> Self {
        Self::new()
    }
}

impl DepTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// 获取所有非系统依赖
    pub fn external_dependencies(&self) -> Vec<&DepTreeNode> {
        self.nodes.iter().filter(|n| !n.is_system).collect()
    }
}

/// 解析 PE 文件的导入表，提取 DLL 列表
pub fn parse_pe_imports(path: &Path) -> Result<Vec<String>> {
    let data = std::fs::read(path)?;
    let pe = goblin::pe::PE::parse(&data)
        .map_err(|e| crate::AnalyzerError::PeParse(e.to_string()))?;

    let dlls: HashSet<String> = pe
        .imports
        .iter()
        .map(|i| i.dll.to_string())
        .collect();

    Ok(dlls.into_iter().collect())
}

/// 判断 DLL 是否是系统自带的
pub fn is_system_dll(name: &str) -> bool {
    SYSTEM_DLLS
        .iter()
        .any(|sys| sys.eq_ignore_ascii_case(name))
}

/// 递归解析依赖链
pub fn resolve_dependency_tree(exe_path: &Path) -> Result<DepTree> {
    let mut tree = DepTree::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // 根节点
    let root_name = exe_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    tree.nodes.push(DepTreeNode {
        path: exe_path.to_path_buf(),
        dll_name: root_name,
        is_system: false,
        depth: 0,
    });
    visited.insert(exe_path.to_path_buf());
    queue.push_back((0usize, exe_path.to_path_buf()));

    while let Some((parent_idx, current_path)) = queue.pop_front() {
        let deps = match parse_pe_imports(&current_path) {
            Ok(d) => d,
            Err(_) => continue, // 无法解析时跳过
        };

        let parent_depth = tree.nodes[parent_idx].depth;

        for dll_name in deps {
            let is_sys = is_system_dll(&dll_name);

            // 搜索 DLL 实际路径
            if let Some(dll_path) = resolve_dll_path(&dll_name, &current_path) {
                let child_idx = tree.nodes.len();
                tree.nodes.push(DepTreeNode {
                    path: dll_path.clone(),
                    dll_name,
                    is_system: is_sys,
                    depth: parent_depth + 1,
                });
                tree.edges.push((parent_idx, child_idx));

                // 递归：只对非系统 DLL 且未访问过的继续解析
                if !is_sys && visited.insert(dll_path.clone()) {
                    queue.push_back((child_idx, dll_path));
                }
            }
        }
    }

    Ok(tree)
}

/// 搜索 DLL 的实际路径
fn resolve_dll_path(dll_name: &str, referencing_exe: &Path) -> Option<std::path::PathBuf> {
    // 搜索顺序：
    // 1. EXE 所在目录
    if let Some(exe_dir) = referencing_exe.parent() {
        let candidate = exe_dir.join(dll_name);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 2. System32
    if let Ok(system_root) = std::env::var("SystemRoot") {
        let system32 = Path::new(&system_root).join("System32").join(dll_name);
        if system32.exists() {
            return Some(system32);
        }

        // 3. SysWOW64（32 位 DLL）
        let syswow64 = Path::new(&system_root).join("SysWOW64").join(dll_name);
        if syswow64.exists() {
            return Some(syswow64);
        }
    }

    // 4. PATH 环境变量
    if let Ok(path_env) = std::env::var("PATH") {
        for dir in path_env.split(';') {
            if dir.is_empty() {
                continue;
            }
            let candidate = Path::new(dir).join(dll_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

/// 在字节切片中搜索字符串模式
pub fn contains_bytes_str(haystack: &[u8], needle: &str) -> bool {
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

// ── 运行时识别 ────────────────────────────────────────────────────────────────

/// 运行时依赖类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RuntimeDependencyType {
    /// Visual C++ 运行时
    VCpp { version: String },
    /// .NET Framework
    DotNet { version: String },
    /// Java 运行时
    Java,
    /// Python 运行时
    Python,
    /// Go 运行时
    Go,
    /// Rust 运行时
    Rust,
    /// Node.js 运行时
    Node,
    /// DirectX
    DirectX,
    /// Visual Basic 运行时
    VisualBasic,
    /// 未知运行时
    Unknown(String),
}

/// 运行时依赖信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeDependency {
    pub runtime_type: RuntimeDependencyType,
    pub dll_name: String,
    pub is_required: bool,
}

/// 识别依赖中的常见运行时
pub fn identify_runtimes(deps: &[String]) -> Vec<RuntimeDependency> {
    let mut runtimes = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for dll in deps {
        let lower = dll.to_lowercase();

        // Visual C++ 运行时
        let vcpp_version = if lower.starts_with("msvcr") && lower.ends_with(".dll") {
            // MSVCRTxx.dll — 旧版 VC++ 运行时
            let ver = &dll[5..dll.len() - 4];
            Some(format!("VC++ {ver}"))
        } else if lower == "vcruntime140.dll" || lower == "vcruntime140_1.dll" {
            Some("VC++ 2015-2022".to_string())
        } else if lower == "vcruntime120.dll" {
            Some("VC++ 2013".to_string())
        } else if lower == "vcruntime110.dll" {
            Some("VC++ 2012".to_string())
        } else if lower == "vcruntime100.dll" || lower == "msvcr100.dll" {
            Some("VC++ 2010".to_string())
        } else if lower == "msvcr90.dll" {
            Some("VC++ 2008".to_string())
        } else if lower == "msvcr80.dll" {
            Some("VC++ 2005".to_string())
        } else if lower == "msvcp140.dll" || lower == "msvcp120.dll" || lower == "msvcp110.dll" {
            Some(format!("VC++ {}", if lower.contains("140") { "2015-2022" } else if lower.contains("120") { "2013" } else { "2012" }))
        } else {
            None
        };

        if let Some(version) = vcpp_version {
            if seen.insert(RuntimeDependencyType::VCpp { version: version.clone() }) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::VCpp { version },
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // .NET Framework
        if lower.starts_with("mscoree.dll") || lower.starts_with("clr") || lower.starts_with("mscorlib") {
            if seen.insert(RuntimeDependencyType::DotNet { version: "4.x".to_string() }) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::DotNet { version: "4.x".to_string() },
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // Java
        if lower == "jvm.dll" || lower == "java.dll" || lower == "jawt.dll" {
            if seen.insert(RuntimeDependencyType::Java) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::Java,
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // Python
        if lower.starts_with("python") && lower.ends_with(".dll") {
            if seen.insert(RuntimeDependencyType::Python) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::Python,
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // Go
        if lower == "go.dll" || lower.contains("_cgo_") {
            if seen.insert(RuntimeDependencyType::Go) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::Go,
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // Rust
        if lower.starts_with("std-") && lower.contains(".dll") {
            if seen.insert(RuntimeDependencyType::Rust) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::Rust,
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // Node.js
        if lower == "node.dll" || lower == "libnode.dll" {
            if seen.insert(RuntimeDependencyType::Node) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::Node,
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // DirectX
        if lower.starts_with("d3d") || lower.starts_with("dxgi") || lower == "dinput8.dll" || lower == "dsound.dll" {
            if seen.insert(RuntimeDependencyType::DirectX) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::DirectX,
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }

        // Visual Basic
        if lower == "msvbvm60.dll" || lower == "msvbvm50.dll" {
            if seen.insert(RuntimeDependencyType::VisualBasic) {
                runtimes.push(RuntimeDependency {
                    runtime_type: RuntimeDependencyType::VisualBasic,
                    dll_name: dll.clone(),
                    is_required: true,
                });
            }
            continue;
        }
    }

    runtimes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_dll_detection() {
        assert!(is_system_dll("kernel32.dll"));
        assert!(is_system_dll("KERNEL32.DLL")); // 大小写不敏感
        assert!(is_system_dll("ntdll.dll"));
        assert!(is_system_dll("user32.dll"));
        assert!(!is_system_dll("myapp_core.dll"));
        assert!(!is_system_dll("libcustom.dll"));
    }

    #[test]
    fn dep_tree_external_filter() {
        let mut tree = DepTree::new();
        tree.nodes.push(DepTreeNode {
            path: std::path::PathBuf::from("app.exe"),
            dll_name: "app.exe".into(),
            is_system: false,
            depth: 0,
        });
        tree.nodes.push(DepTreeNode {
            path: std::path::PathBuf::from("kernel32.dll"),
            dll_name: "kernel32.dll".into(),
            is_system: true,
            depth: 1,
        });
        tree.nodes.push(DepTreeNode {
            path: std::path::PathBuf::from("custom.dll"),
            dll_name: "custom.dll".into(),
            is_system: false,
            depth: 1,
        });

        let external = tree.external_dependencies();
        assert_eq!(external.len(), 2); // app.exe + custom.dll
    }

    #[test]
    fn identify_vcpp_runtime() {
        let deps = vec!["vcruntime140.dll".to_string(), "msvcp140.dll".to_string()];
        let runtimes = identify_runtimes(&deps);
        assert!(!runtimes.is_empty());
        assert!(runtimes.iter().any(|r| matches!(&r.runtime_type, RuntimeDependencyType::VCpp { .. })));
    }

    #[test]
    fn identify_dotnet_runtime() {
        let deps = vec!["mscoree.dll".to_string()];
        let runtimes = identify_runtimes(&deps);
        assert!(!runtimes.is_empty());
        assert!(runtimes.iter().any(|r| matches!(&r.runtime_type, RuntimeDependencyType::DotNet { .. })));
    }

    #[test]
    fn identify_java_runtime() {
        let deps = vec!["jvm.dll".to_string()];
        let runtimes = identify_runtimes(&deps);
        assert!(!runtimes.is_empty());
        assert!(runtimes.iter().any(|r| r.runtime_type == RuntimeDependencyType::Java));
    }

    #[test]
    fn identify_python_runtime() {
        let deps = vec!["python311.dll".to_string()];
        let runtimes = identify_runtimes(&deps);
        assert!(!runtimes.is_empty());
        assert!(runtimes.iter().any(|r| r.runtime_type == RuntimeDependencyType::Python));
    }

    #[test]
    fn identify_directx_runtime() {
        let deps = vec!["d3d11.dll".to_string(), "dxgi.dll".to_string()];
        let runtimes = identify_runtimes(&deps);
        assert!(!runtimes.is_empty());
        assert!(runtimes.iter().any(|r| r.runtime_type == RuntimeDependencyType::DirectX));
    }

    #[test]
    fn identify_no_runtime_for_system_dlls() {
        let deps = vec!["kernel32.dll".to_string(), "user32.dll".to_string()];
        let runtimes = identify_runtimes(&deps);
        assert!(runtimes.is_empty());
    }
}
