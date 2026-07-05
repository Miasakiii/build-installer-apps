//! Inno Setup 脚本生成器
//!
//! 从 PackageManifest 生成 .iss 脚本文件，并可选调用 Inno Setup 编译。

use crate::error::Result;
use camino::Utf8PathBuf;
use packforge_core::{
    InstallAction, PackageEntry, PackageManifest, UninstallAction,
};
use std::path::Path;

/// Inno Setup 编译选项
#[derive(Debug, Clone)]
pub struct InnoCompileOptions {
    /// 是否静默编译
    pub silent: bool,
    /// 输出目录
    pub output_dir: Option<Utf8PathBuf>,
    /// 是否签名（需要 signtool.exe）
    pub sign: bool,
}

impl Default for InnoCompileOptions {
    fn default() -> Self {
        Self {
            silent: true,
            output_dir: None,
            sign: false,
        }
    }
}

/// 从 PackageManifest 生成 Inno Setup 脚本
pub fn generate_inno_script(manifest: &PackageManifest) -> String {
    let mut script = String::new();

    // [Setup] 段
    script.push_str(&generate_setup_section(manifest));
    script.push('\n');

    // [Files] 段
    script.push_str(&generate_files_section(manifest));
    script.push('\n');

    // [Icons] 段
    script.push_str(&generate_icons_section(manifest));
    script.push('\n');

    // [Registry] 段
    if !manifest.registry.is_empty() {
        script.push_str(&generate_registry_section(manifest));
        script.push('\n');
    }

    // [Run] 段
    script.push_str(&generate_run_section(manifest));
    script.push('\n');

    // [UninstallRun] 段
    script.push_str(&generate_uninstall_run_section(manifest));
    script.push('\n');

    // [UninstallDelete] 段
    script.push_str(&generate_uninstall_delete_section(manifest));
    script.push('\n');

    script
}

/// 生成 [Setup] 段
fn generate_setup_section(manifest: &PackageManifest) -> String {
    let app_name = &manifest.name;
    let app_version = &manifest.version;
    let publisher = manifest.publisher.as_deref().unwrap_or("Unknown");
    let output_base = format!("{}-{}-setup", app_name.to_lowercase().replace(' ', "-"), app_version);

    format!(
        r#"[Setup]
AppName={app_name}
AppVersion={app_version}
AppPublisher={publisher}
DefaultDirName={{autopf}}\{app_name}
DefaultGroupName={app_name}
OutputDir=output
OutputBaseFilename={output_base}
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayName={app_name}
UninstallDisplayIcon={{app}}\{app_name}.exe
"#
    )
}

/// 生成 [Files] 段
fn generate_files_section(manifest: &PackageManifest) -> String {
    let mut section = String::from("[Files]\n");

    for entry in &manifest.entries {
        match entry {
            PackageEntry::File { source, dest, .. } => {
                section.push_str(&format!(
                    "Source: \"{}\"; DestDir: \"{}\"; Flags: ignoreversion\n",
                    escape_inno_path(source.as_str()),
                    extract_dir(dest)
                ));
            }
            PackageEntry::Directory { source, dest } => {
                section.push_str(&format!(
                    "Source: \"{}\\*\"; DestDir: \"{}\"; Flags: recursesubdirs ignoreversion\n",
                    escape_inno_path(source.as_str()),
                    escape_inno_path(dest.as_str())
                ));
            }
        }
    }

    section
}

/// 生成 [Icons] 段
fn generate_icons_section(manifest: &PackageManifest) -> String {
    let mut section = String::from("[Icons]\n");
    let app_name = &manifest.name;

    // 从安装动作中提取快捷方式
    for action in &manifest.install_actions {
        if let InstallAction::CreateShortcut { target, shortcut, .. } = action {
            let shortcut_str = shortcut.as_str();
            if shortcut_str.contains("Start Menu") {
                section.push_str(&format!(
                    "Name: \"{{group}}\\{}\"; Filename: \"{}\"\n",
                    escape_inno_value(app_name),
                    escape_inno_path(target.as_str())
                ));
            } else if shortcut_str.contains("Desktop") {
                section.push_str(&format!(
                    "Name: \"{{autodesktop}}\\{}\"; Filename: \"{}\"\n",
                    escape_inno_value(app_name),
                    escape_inno_path(target.as_str())
                ));
            }
        }
    }

    // 如果没有快捷方式，创建默认的开始菜单快捷方式
    if !manifest
        .install_actions
        .iter()
        .any(|a| matches!(a, InstallAction::CreateShortcut { .. }))
    {
        section.push_str(&format!(
            "Name: \"{{group}}\\{}\"; Filename: \"{{app}}\\{}.exe\"\n",
            escape_inno_value(app_name),
            escape_inno_value(app_name)
        ));
        section.push_str(&format!(
            "Name: \"{{group}}\\卸载 {}\"; Filename: \"{{uninstallexe}}\"\n",
            escape_inno_value(app_name)
        ));
    }

    section
}

/// 生成 [Registry] 段
fn generate_registry_section(manifest: &PackageManifest) -> String {
    let mut section = String::from("[Registry]\n");

    for entry in &manifest.registry {
        let root = match entry.hive {
            packforge_core::RegistryHive::HKLM => "HKLM",
            packforge_core::RegistryHive::HKCU => "HKCU",
            packforge_core::RegistryHive::HKCR => "HKCR",
            packforge_core::RegistryHive::HKU => "HKU",
            packforge_core::RegistryHive::HKCC => "HKCC",
        };

        let value_type = match entry.value_type {
            packforge_core::RegistryValueType::String => "string",
            packforge_core::RegistryValueType::ExpandString => "expandsz",
            packforge_core::RegistryValueType::Binary => "binary",
            packforge_core::RegistryValueType::DWord => "dword",
            packforge_core::RegistryValueType::QWord => "qword",
            packforge_core::RegistryValueType::MultiString => "multisz",
            packforge_core::RegistryValueType::Unknown => "string",
        };

        let value_name = entry
            .value_name
            .as_deref()
            .unwrap_or("");

        section.push_str(&format!(
            "Root: {}; Subkey: \"{}\"; ValueType: {}; ValueName: \"{}\"; ValueData: \"{}\"\n",
            root,
            escape_inno_path(&entry.key_path),
            value_type,
            escape_inno_value(value_name),
            escape_inno_value(&entry.value_data)
        ));
    }

    section
}

/// 生成 [Run] 段
fn generate_run_section(manifest: &PackageManifest) -> String {
    let mut section = String::from("[Run]\n");

    // 从安装动作中提取运行命令
    for action in &manifest.install_actions {
        match action {
            InstallAction::RunCommand { command, args } => {
                let args_str = args.join(" ");
                section.push_str(&format!(
                    "Filename: \"{}\"; Parameters: \"{}\"; Flags: postinstall nowait\n",
                    escape_inno_path(command),
                    escape_inno_value(&args_str)
                ));
            }
            InstallAction::RegisterCom { dll_path } => {
                section.push_str(&format!(
                    "Filename: \"regsvr32\"; Parameters: \"/s \\\"{}\\\"\"; Flags: runhidden\n",
                    escape_inno_path(dll_path.as_str())
                ));
            }
            _ => {}
        }
    }

    // 默认：安装后启动程序
    section.push_str(&format!(
        "Filename: \"{{app}}\\{}.exe\"; Description: \"启动 {}\"; Flags: postinstall nowait\n",
        escape_inno_value(&manifest.name),
        escape_inno_value(&manifest.name)
    ));

    section
}

/// 生成 [UninstallRun] 段
fn generate_uninstall_run_section(manifest: &PackageManifest) -> String {
    let mut section = String::from("[UninstallRun]\n");

    for action in &manifest.uninstall_actions {
        match action {
            UninstallAction::UnregisterCom { dll_path } => {
                section.push_str(&format!(
                    "Filename: \"regsvr32\"; Parameters: \"/s /u \\\"{}\\\"\"; Flags: runhidden\n",
                    escape_inno_path(dll_path.as_str())
                ));
            }
            UninstallAction::RunCommand { command, args } => {
                let args_str = args.join(" ");
                section.push_str(&format!(
                    "Filename: \"{}\"; Parameters: \"{}\"; Flags: runhidden\n",
                    escape_inno_path(command),
                    escape_inno_value(&args_str)
                ));
            }
            _ => {}
        }
    }

    section
}

/// 生成 [UninstallDelete] 段
fn generate_uninstall_delete_section(manifest: &PackageManifest) -> String {
    let mut section = String::from("[UninstallDelete]\n");

    for action in &manifest.uninstall_actions {
        match action {
            UninstallAction::RemoveDirectory(path) => {
                section.push_str(&format!(
                    "Type: filesandordirs; Name: \"{}\"\n",
                    escape_inno_path(path.as_str())
                ));
            }
            UninstallAction::RemoveFile(path) => {
                section.push_str(&format!(
                    "Type: files; Name: \"{}\"\n",
                    escape_inno_path(path.as_str())
                ));
            }
            UninstallAction::RemoveShortcut(path) => {
                section.push_str(&format!(
                    "Type: files; Name: \"{}\"\n",
                    escape_inno_path(path.as_str())
                ));
            }
            _ => {}
        }
    }

    section
}

/// 转义 Inno Setup 路径中的特殊字符
///
/// 安全说明：防止路径中的特殊字符导致脚本注入。
/// 将反斜杠转换为正斜杠，并转义双引号。
fn escape_inno_path(path: &str) -> String {
    path.replace('\\', "/")
        .replace('"', "\"\"")
        .replace('\'', "''")
}

/// 转义 Inno Setup 值中的特殊字符
///
/// 安全说明：防止值中的特殊字符导致脚本注入。
/// Inno Setup 使用双引号包围值，内部的双引号需要转义。
fn escape_inno_value(value: &str) -> String {
    value
        .replace('"', "\"\"")  // 双引号转义
        .replace('\\', "\\\\")  // 反斜杠转义
        .replace('\n', "\\n")   // 换行符转义
        .replace('\r', "\\r")   // 回车符转义
        .replace('\t', "\\t")   // 制表符转义
}

/// 从路径中提取目录部分
fn extract_dir(path: &Utf8PathBuf) -> String {
    path.parent()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "{app}".to_string())
}

/// 保存 Inno Setup 脚本到文件
pub fn save_inno_script(manifest: &PackageManifest, output_path: &Path) -> Result<()> {
    let script = generate_inno_script(manifest);
    std::fs::write(output_path, script)?;
    Ok(())
}

/// 编译 Inno Setup 脚本
///
/// 查找系统中的 iscc.exe 并调用编译。
pub fn compile_inno_setup(iss_path: &Path, options: &InnoCompileOptions) -> Result<Utf8PathBuf> {
    // 查找 iscc.exe
    let iscc_path = find_iscc_exe()
        .ok_or_else(|| crate::PackagerError::Compile("未找到 Inno Setup 编译器 (iscc.exe)".into()))?;

    let mut cmd = std::process::Command::new(&iscc_path);

    if options.silent {
        cmd.arg("/Q");
    }

    if let Some(ref output_dir) = options.output_dir {
        cmd.arg(format!("/O{}", output_dir.as_str()));
    }

    cmd.arg(iss_path.as_os_str());

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::PackagerError::Compile(format!(
            "Inno Setup 编译失败: {stderr}"
        )));
    }

    // 返回输出的 .exe 路径
    let exe_name = iss_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("setup");
    let output_dir = options
        .output_dir
        .as_ref()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| {
            iss_path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        });

    Ok(Utf8PathBuf::from(format!("{output_dir}\\{exe_name}.exe")))
}

/// 查找 Inno Setup 编译器路径
pub fn find_iscc_exe() -> Option<String> {
    // 常见安装路径
    let paths = [
        "C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe",
        "C:\\Program Files\\Inno Setup 6\\ISCC.exe",
        "C:\\Program Files (x86)\\Inno Setup 5\\ISCC.exe",
        "C:\\Program Files\\Inno Setup 5\\ISCC.exe",
    ];

    for path in &paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    // 尝试从 PATH 中查找
    std::process::Command::new("where")
        .arg("iscc")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.lines().next().unwrap_or("").to_string())
            } else {
                None
            }
        })
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manifest() -> PackageManifest {
        PackageManifest {
            name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            publisher: Some("Test Publisher".to_string()),
            description: None,
            license: None,
            icon: None,
            entries: vec![
                PackageEntry::File {
                    source: Utf8PathBuf::from("C:/Program Files/TestApp/bin/main.exe"),
                    dest: Utf8PathBuf::from("{app}\\bin\\main.exe"),
                    overwrite: true,
                },
                PackageEntry::File {
                    source: Utf8PathBuf::from("C:/Program Files/TestApp/lib/core.dll"),
                    dest: Utf8PathBuf::from("{app}\\lib\\core.dll"),
                    overwrite: true,
                },
            ],
            registry: vec![],
            install_actions: vec![],
            uninstall_actions: vec![
                UninstallAction::RemoveDirectory(Utf8PathBuf::from("{app}")),
            ],
        }
    }

    #[test]
    fn generate_inno_script_has_required_sections() {
        let manifest = create_test_manifest();
        let script = generate_inno_script(&manifest);

        assert!(script.contains("[Setup]"));
        assert!(script.contains("[Files]"));
        assert!(script.contains("[Icons]"));
        assert!(script.contains("[Run]"));
        assert!(script.contains("[UninstallDelete]"));
    }

    #[test]
    fn generate_inno_script_has_correct_app_info() {
        let manifest = create_test_manifest();
        let script = generate_inno_script(&manifest);

        assert!(script.contains("AppName=TestApp"));
        assert!(script.contains("AppVersion=1.0.0"));
        assert!(script.contains("AppPublisher=Test Publisher"));
    }

    #[test]
    fn generate_inno_script_has_file_entries() {
        let manifest = create_test_manifest();
        let script = generate_inno_script(&manifest);

        assert!(script.contains("main.exe"));
        assert!(script.contains("core.dll"));
    }

    #[test]
    fn extract_dir_works() {
        let path = Utf8PathBuf::from("{app}\\bin\\main.exe");
        let dir = extract_dir(&path);
        assert_eq!(dir, "{app}\\bin");
    }

    #[test]
    fn default_compile_options() {
        let opts = InnoCompileOptions::default();
        assert!(opts.silent);
        assert!(!opts.sign);
    }
}
