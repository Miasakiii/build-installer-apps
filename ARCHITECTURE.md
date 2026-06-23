# PackForge 架构设计

## 系统架构总览

```text
┌─────────────────────────────────────────────────────────────────┐
│                      CLI (packforge-cli)                         │
│  list / scan / analyze / extract / package / report / batch-*    │
└──────┬──────────────┬──────────────┬──────────────┬─────────────┘
       │              │              │              │
       ▼              ▼              ▼              ▼
┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐
│  Scanner   │ │  Analyzer  │ │  Packager  │ │  Reporter  │
│            │ │            │ │            │ │            │
│ · 文件扫描  │ │ · 依赖分析  │ │ · Inno Setup│ │ · JSON     │
│ · 注册表   │ │ · 可移植性  │ │ · Manifest │ │ · CSV      │
│ · 服务枚举  │ │ · 运行时   │ │ · Portable │ │ · 批量操作  │
│ · COM 组件  │ │ · 框架识别  │ │            │ │            │
│ · 环境变量  │ │            │ │            │ │            │
│ · 快照对比  │ │            │ │            │ │            │
└──────┬─────┘ └──────┬─────┘ └──────┬─────┘ └──────┬─────┘
       │              │              │              │
       ▼              ▼              ▼              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Core (packforge-core)                          │
│  SoftwareProfile / FileEntry / RegistryEntry / PackageManifest    │
└─────────────────────────────────────────────────────────────────┘
```

## 核心数据模型

### SoftwareProfile — 软件画像

```rust
/// 一个已安装软件的完整画像
pub struct SoftwareProfile {
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub install_dir: PathBuf,
    pub uninstall_string: Option<String>,
    pub install_date: Option<NaiveDate>,
    
    // 扫描结果
    pub files: Vec<FileEntry>,
    pub registry_entries: Vec<RegistryEntry>,
    pub services: Vec<ServiceEntry>,
    pub com_components: Vec<ComEntry>,
    pub env_vars: Vec<EnvVarEntry>,
    pub shortcuts: Vec<ShortcutEntry>,
}

pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub hash: Option<[u8; 32]>, // SHA-256, 计算时填充
    pub file_type: FileType,
    pub is_executable: bool,
    pub dependencies: Vec<String>, // 导入的 DLL 列表
}

pub enum FileType {
    Executable,
    DynamicLibrary,
    StaticLibrary,
    ConfigFile,
    DataFile,
    Documentation,
    Plugin,
    Driver,
    Unknown,
}

pub struct RegistryEntry {
    pub hive: RegistryHive,     // HKLM / HKCU / HKCR
    pub key_path: String,
    pub value_name: Option<String>,
    pub value_type: RegistryValueType,
    pub value_data: String,
    pub is_system_critical: bool, // 是否是程序运行必需
}
```

### AnalysisResult — 分析结果

```rust
pub struct AnalysisResult {
    pub software: SoftwareProfile,
    
    // 分类结果
    pub core_files: Vec<FileEntry>,         // 程序运行必需
    pub optional_files: Vec<FileEntry>,     // 可选组件
    pub user_data_files: Vec<FileEntry>,    // 用户生成的数据
    pub temp_files: Vec<FileEntry>,         // 可清理
    
    pub core_registry: Vec<RegistryEntry>,  // 程序运行必需
    pub user_registry: Vec<RegistryEntry>,  // 用户偏好设置
    
    // 依赖分析
    pub external_dependencies: Vec<Dependency>,  // 外部依赖（VC++ Runtime 等）
    pub install_framework: Option<InstallFramework>, // 识别出的安装框架
    pub runtime: Option<RuntimeType>,            // 识别出的运行时环境
    
    // 可移植性评估
    pub portability_score: f32,  // 0.0 ~ 1.0
    pub portability_issues: Vec<PortabilityIssue>,
}

/// 安装框架类型（打包工具）
pub enum InstallFramework {
    Nsis,
    InnoSetup,
    WiX,
    InstallShield,
    AdvancedInstaller,
    Electron,     // electron-builder / electron-forge
    Msix,
    Custom(String),
    Unknown,
}

/// 运行时环境类型（非安装框架，但影响便携化可行性）
pub enum RuntimeType {
    DotNet { version: String, self_contained: bool },
    Java { has_embedded_jre: bool },
    Python { has_embedded_interpreter: bool },
    Go,
    Rust,
    Node,
    Native,
}

pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub is_system: bool,        // 是否系统自带
    pub download_url: Option<String>, // 获取地址
}

pub enum PortabilityIssue {
    HardcodedPath(PathBuf),        // 路径硬编码
    RequiresService(String),       // 依赖 Windows 服务
    RequiresComRegistration(String), // 需要 COM 注册
    WritesToRegistry(String),      // 必须写注册表
    RequiresAdmin,                 // 需要管理员权限
    UsesGlobalMutex(String),       // 使用全局互斥量
}
```

### PackageManifest — 打包清单

```rust
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    pub publisher: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub icon: Option<PathBuf>,
    
    pub entries: Vec<PackageEntry>,
    pub registry: Vec<RegistryEntry>,
    pub install_actions: Vec<InstallAction>,
    pub uninstall_actions: Vec<UninstallAction>,
}

pub enum PackageEntry {
    File {
        source: PathBuf,        // 源路径
        dest: PathBuf,          // 安装后路径
        overwrite: bool,
    },
    Directory {
        source: PathBuf,
        dest: PathBuf,
    },
}

pub enum InstallAction {
    CreateDirectory(PathBuf),
    SetPermissions { path: PathBuf, permissions: String },
    RegisterCom { dll_path: PathBuf },
    CreateService { name: String, exe_path: PathBuf, start_type: ServiceStartType },
    SetEnvironmentVariable { name: String, value: String, scope: EnvScope },
    CreateShortcut { target: PathBuf, shortcut: PathBuf, icon: Option<PathBuf> },
    RunCommand { command: String, args: Vec<String>, timing: ActionTiming },
}
```

## 模块详细设计

### 1. Scanner 模块 — 扫描引擎

**职责**：收集已安装软件的完整状态快照。

#### 1.1 软件发现

```rust
// 从注册表枚举已安装软件
// HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
// HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall
// HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall
pub fn discover_installed_software() -> Vec<SoftwareInfo>;

// 根据名称/路径精确匹配
pub fn find_software(query: &str) -> Vec<SoftwareInfo>;
```

#### 1.2 深度扫描

```rust
// 扫描安装目录下所有文件
pub fn scan_install_dir(path: &Path) -> Vec<FileEntry>;

// 扫描注册表中与该软件相关的所有条目
// 策略：按软件名、安装路径、Publisher 等关键词搜索
pub fn scan_registry_entries(software: &SoftwareInfo) -> Vec<RegistryEntry>;

// 扫描关联的 Windows 服务
pub fn scan_services(software: &SoftwareInfo) -> Vec<ServiceEntry>;

// 扫描 COM 组件注册
pub fn scan_com_components(software: &SoftwareInfo) -> Vec<ComEntry>;

// 扫描环境变量
pub fn scan_env_vars(software: &SoftwareInfo) -> Vec<EnvVarEntry>;

// 扫描快捷方式
pub fn scan_shortcuts(software: &SoftwareInfo) -> Vec<ShortcutEntry>;
```

#### 1.3 安装前后对比（增量模式）

```rust
// 安装前拍摄快照
pub fn take_snapshot() -> SystemSnapshot;

// 安装后拍摄快照，对比差异
pub fn diff_snapshots(before: &SystemSnapshot, after: &SystemSnapshot) -> SystemDiff;

pub struct SystemSnapshot {
    pub files: HashMap<PathBuf, FileMeta>,
    pub registry: Vec<RegistryEntry>,
    pub services: Vec<ServiceInfo>,
    pub env_vars: HashMap<String, String>,
    pub timestamp: Instant,
}
```

### 2. Analyzer 模块 — 智能分析

**职责**：从原始扫描数据中提取语义信息。

#### 2.1 文件分类

```rust
// 基于扩展名、路径、内容的启发式分类
pub fn classify_files(files: &[FileEntry]) -> ClassifiedFiles;

// 策略：
// - *.exe, *.dll → Executable/DLL
// - *.config, *.ini, *.json, *.xml, *.yaml → Config
// - *.log, *.tmp, cache/* → Temp/UserData
// - docs/*, readme*, license* → Documentation
// - plugins/*, addons/* → Optional
```

#### 2.2 依赖分析

```rust
// 分析 PE 文件的导入表
pub fn analyze_pe_dependencies(exe_path: &Path) -> Vec<String>;

// 识别常见运行时依赖
pub fn detect_runtime_dependencies(deps: &[String]) -> Vec<Dependency>;
// VC++ Runtime, .NET Framework, Java Runtime, DirectX, etc.

// 识别原安装框架
pub fn detect_install_framework(install_dir: &Path, uninstaller: Option<&Path>) -> Option<InstallFramework>;
// 特征：
// - NSIS: uninstaller.exe 内含 "Nullsoft" 字符串
// - Inno Setup: unins000.exe, unins000.dat
// - WiX: 有 MSI 产品代码
// - InstallShield: setup.ini, data1.cab
```

#### 2.3 可移植性评估

```rust
pub fn assess_portability(profile: &SoftwareProfile) -> PortabilityReport;

// 检查项：
// - 是否有硬编码的绝对路径
// - 是否依赖 Windows 服务
// - 是否有 COM 注册
// - 是否写 HKLM 注册表（需要管理员）
// - 是否使用全局互斥量/管道
// - 是否有内核驱动
```

#### 2.4 用户数据剥离

```rust
pub fn separate_user_data(profile: &SoftwareProfile) -> SeparationResult;

// 策略：
// - %AppData%/{软件名} 下的文件 → 可能是用户配置
// - 安装目录下的 *.log, *.dat (非程序必需) → 用户数据
// - 注册表 HKCU 下的条目 → 用户偏好
// - 安装目录下的 savegames/, projects/, documents/ → 用户数据
// - 数据库文件 (*.db, *.sqlite) → 需要人工确认
```

### 3. Packager 模块 — 打包输出

**职责**：根据分析结果生成可安装的包。

#### 3.1 便携化打包（P1 优先实现）

```rust
pub fn create_portable_package(
    profile: &SoftwareProfile,
    analysis: &AnalysisResult,
    output: &Path,
) -> Result<PackageReport>;

// 输出：
// {name}-{version}-portable/
//   ├── {name}/              # 程序文件
//   │   ├── bin/
//   │   ├── lib/
//   │   ├── config/          # 默认配置（非用户数据）
//   │   └── ...
//   ├── launcher.bat         # Windows 启动脚本
//   ├── launcher.sh          # Linux 启动脚本
//   ├── README.md            # 使用说明
//   └── manifest.json        # 包元数据
```

#### 3.2 Inno Setup 脚本生成

```rust
pub fn generate_inno_script(manifest: &PackageManifest) -> String;

// 生成 .iss 文件内容：
// [Setup]
// AppName={name}
// AppVersion={version}
// DefaultDirName={autopf}\{name}
// ...
//
// [Files]
// Source: "{source}\*"; DestDir: "{app}"; Flags: recursesubdirs
//
// [Registry]
// ...从 manifest.registry 生成...
//
// [Run]
// ...从 manifest.install_actions 生成...
```

#### 3.3 NSIS 脚本生成

```rust
pub fn generate_nsis_script(manifest: &PackageManifest) -> String;
```

#### 3.4 WiX/MSI 项目生成

```rust
pub fn generate_wix_project(manifest: &PackageManifest) -> WixProject;
```

### 4. CLI 设计

```text
USAGE:
    packforge [OPTIONS] <COMMAND>

COMMANDS:
    list                    📋 列出已安装软件
    scan <软件名>           🔍 深度扫描指定软件
    analyze <软件名>        📊 分析软件结构 & 可移植性
    extract <软件名>        📦 提取为便携化包
    package <软件名>        🛠️ 生成安装程序
    snapshot                📸 拍摄系统快照
    diff <before> <after>   🔄 对比快照差异
    report                  📋 导出软件清单报告
    batch-extract           📦 批量提取多个软件
    batch-package           🛠️ 批量生成安装程序

OPTIONS:
    -o, --output <PATH>     输出路径
    -f, --format <FORMAT>   输出格式 (table|json|yaml)
    -v, --verbose           详细输出
    --no-color              禁用彩色输出
    -h, --help              帮助
```

**package 子命令参数**：

```text
packforge package <NAME> [OPTIONS]
    --pkg-format <FORMAT>       安装程序格式 (inno|nsis|wix)
    --app-name <NAME>           应用名称（覆盖检测值）
    --publisher <NAME>          发布者（覆盖检测值）
    --app-version <VERSION>     版本（覆盖检测值）
    --no-compile                只生成脚本，不编译
    --no-desktop-shortcut       不创建桌面快捷方式
    --no-start-menu             不创建开始菜单快捷方式
```

## 技术难点与对策

### 难点 1：注册表条目归属判断

**问题**：注册表中数千个条目，如何判断哪些属于目标软件？

**对策**：

1. 精确匹配：从 Uninstall 键获取 InstallLocation，匹配路径前缀
2. 模糊匹配：按软件名、Publisher 搜索
3. 增量对比：安装前后快照 diff（最准确）
4. PE 分析：从 EXE/DLL 中提取公司名、产品名作为搜索关键词

### 难点 2：DLL 依赖链追踪

**问题**：一个 EXE 依赖 A.dll，A.dll 又依赖 B.dll...

**对策**：

```rust
// 递归解析 PE 导入表
pub fn resolve_dependency_chain(exe_path: &Path) -> DependencyGraph;

// 使用 petgraph 构建依赖图
// 已知系统 DLL 列表过滤（kernel32.dll, user32.dll 等）
// 检查 WinSxS 和 SxS manifest
```

### 难点 3：用户数据 vs 程序数据

**问题**：`config.ini` 是默认配置还是用户修改后的？

**对策**：

1. 哈希对比：与安装包内原始文件对比（如果有）
2. 时间戳：安装时间之后修改的文件可能是用户数据
3. 内容分析：包含用户名、路径、个性化设置的 → 用户数据
4. 保守策略：不确定的标记为"需人工确认"

### 难点 4：安装框架逆向

**问题**：如何识别原始安装包用的什么打包工具？

**对策**：

```rust
// 特征指纹匹配
const NSIS_SIGNATURE: &[u8] = b"Nullsoft";  // 在 uninstaller 中搜索
const INNO_MARKER: &str = "Inno Setup";
const WIX_MARKER: &str = "WiX";

// 卸载程序特征
// - unins000.exe / unins001.dat → Inno Setup
// - unins*.exe 内含 NSIS → NSIS
// - MsiExec.exe /{GUID} → WiX/MSI
// - setup.exe / InstallShield → InstallShield
```

## 安全与合规

1. **只读扫描**：扫描阶段不修改系统任何内容
2. **用户确认**：打包前展示完整清单，用户确认后才执行
3. **免责声明**：明确标注"仅供个人备份使用，不得用于分发商业软件"
4. **日志审计**：所有操作记录到日志文件
5. **权限最小化**：普通用户权限运行，需要管理员时明确提示
