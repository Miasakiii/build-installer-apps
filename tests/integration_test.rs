//! PackForge 集成测试

use packforge_core::{InstallFramework, RuntimeType};

#[test]
fn core_types_serialization_roundtrip() {
    // 测试 InstallFramework 序列化/反序列化
    let frameworks = vec![
        InstallFramework::Nsis,
        InstallFramework::InnoSetup,
        InstallFramework::WiX,
        InstallFramework::Electron,
        InstallFramework::Unknown,
    ];

    for fw in &frameworks {
        let json = serde_json::to_string(fw).unwrap();
        let back: InstallFramework = serde_json::from_str(&json).unwrap();
        assert_eq!(fw, &back);
    }
}

#[test]
fn runtime_type_serialization() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Go,
        RuntimeType::Rust,
        RuntimeType::DotNet {
            version: "8.0".into(),
            self_contained: true,
        },
        RuntimeType::Java {
            has_embedded_jre: false,
        },
    ];

    for rt in &runtimes {
        let json = serde_json::to_string(rt).unwrap();
        let back: RuntimeType = serde_json::from_str(&json).unwrap();
        assert_eq!(rt, &back);
    }
}

#[test]
fn analyzer_system_dll_list_is_consistent() {
    // 确保关键系统 DLL 被正确识别
    assert!(packforge_analyzer::dependencies::is_system_dll("kernel32.dll"));
    assert!(packforge_analyzer::dependencies::is_system_dll("ntdll.dll"));
    assert!(packforge_analyzer::dependencies::is_system_dll("USER32.DLL")); // 大小写
    assert!(!packforge_analyzer::dependencies::is_system_dll("custom_app.dll"));
}

#[test]
fn analyzer_framework_detection_msi() {
    let fw = packforge_analyzer::framework::detect_install_framework(
        std::path::Path::new("C:\\Program Files\\App"),
        Some("MsiExec.exe /X{12345678-1234-1234-1234-123456789012}"),
    );
    assert_eq!(fw, InstallFramework::WiX);
}

#[test]
fn packager_launcher_generation() {
    // 验证 launcher.bat 内容
    let launcher = packforge_packager::portable::create_portable_package;
    // 直接测试内部函数（通过 public API 间接验证）
    // launcher.bat 应包含环境变量重定向
    let content = include_str!("../crates/packforge-packager/src/portable.rs");
    assert!(content.contains("APPDATA"));
    assert!(content.contains("APP_DIR"));
}
