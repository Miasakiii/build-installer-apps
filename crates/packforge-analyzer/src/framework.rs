//! 安装框架识别模块
//!
//! 通过特征指纹识别软件使用的原始安装框架。

use std::path::Path;

use packforge_core::InstallFramework;

/// 识别安装框架
///
/// 检测策略：
/// 1. 检查 UninstallString 中的特征
/// 2. 检查卸载程序 PE 中的签名
/// 3. 检查安装目录文件特征
/// 4. 检查注册表自定义字段
pub fn detect_install_framework(
    install_dir: &Path,
    uninstall_string: Option<&str>,
) -> InstallFramework {
    // 1. MSI 检测
    if let Some(us) = uninstall_string
        && us.contains("MsiExec") {
            return InstallFramework::WiX;
        }

    // 2. 卸载程序特征检测
    if let Some(framework) = detect_by_uninstaller(install_dir) {
        return framework;
    }

    // 3. 文件特征检测
    if install_dir.join("resources").join("app.asar").exists() {
        return InstallFramework::Electron;
    }

    // 4. Inno Setup 注册表特征
    if let Some(us) = uninstall_string
        && us.contains("unins") {
            return InstallFramework::InnoSetup;
        }

    InstallFramework::Unknown
}

/// 通过卸载程序文件特征识别
fn detect_by_uninstaller(install_dir: &Path) -> Option<InstallFramework> {
    // Inno Setup: unins000.exe + unins000.dat
    if install_dir.join("unins000.exe").exists() {
        // 进一步验证：PE 内含 "Inno Setup"
        if let Ok(data) = std::fs::read(install_dir.join("unins000.exe"))
            && contains_bytes(&data, b"Inno Setup") {
                return Some(InstallFramework::InnoSetup);
            }
        return Some(InstallFramework::InnoSetup); // 文件存在也足够判断
    }

    // NSIS: uninstall.exe 内含 "Nullsoft"
    for name in &["uninstall.exe", "uninst.exe"] {
        let path = install_dir.join(name);
        if path.exists()
            && let Ok(data) = std::fs::read(&path)
                && contains_bytes(&data, b"Nullsoft") {
                    return Some(InstallFramework::Nsis);
                }
    }

    // InstallShield: setup.ini + data1.cab
    if install_dir.join("setup.ini").exists() && install_dir.join("data1.cab").exists() {
        return Some(InstallFramework::InstallShield);
    }

    None
}

/// 在字节切片中搜索模式
fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_bytes_finds_pattern() {
        assert!(contains_bytes(b"Hello Nullsoft World", b"Nullsoft"));
        assert!(!contains_bytes(b"Hello World", b"Nullsoft"));
    }

    #[test]
    fn msi_detected_from_uninstall_string() {
        let framework = detect_install_framework(
            Path::new("C:\\Program Files\\App"),
            Some("MsiExec.exe /X{GUID}"),
        );
        assert_eq!(framework, InstallFramework::WiX);
    }
}
