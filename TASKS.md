# PackForge 任务分解书

> 每个任务标注：阶段 / 优先级 / 预估复杂度 / 依赖 / 交付物
> Agent 可按顺序领取并执行
>
> **注意**：P1 聚焦 MVP（便携化提取），P3 只实现 Inno Setup 一种安装程序格式。NSIS/WiX 后续按需添加。

---

## P0：技术调研 & 项目搭建

### T0.1 — 项目初始化

- **优先级**：P0 🔴
- **复杂度**：低
- **依赖**：无
- **交付物**：
  - [x] Cargo workspace 项目结构
  - [x] 5 个 crate 骨架（core/scanner/analyzer/packager/cli）
  - [x] 基础 CI 配置（GitHub Actions: build + test + clippy）
  - [x] README.md 基础文档

**执行步骤**：

```bash
mkdir packforge && cd packforge
cargo init --name packforge
# 拆分 workspace
# 创建 crates/ 下各子 crate
# 配置 Cargo.toml workspace members
# 编写各 crate 的 lib.rs / main.rs 骨架
```

---

### T0.2 — Windows 已安装软件枚举调研

- **优先级**：P0 🔴
- **复杂度**：中
- **依赖**：无
- **交付物**：
  - [x] 文档：`docs/research/windows-software-registry.md`
  - [x] 代码：`packforge-scanner` 中的 `discovery` 模块原型

**调研内容**：

1. 注册表 Uninstall 键的完整结构（字段含义）
2. 32 位 vs 64 位软件的注册表路径差异
3. MSI 安装的软件与非 MSI 的区别
4. Windows Store / MSIX 应用的发现方式
5. 绿色软件（无安装记录）的发现策略

**输出格式**：

```markdown
## 注册表路径
- HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{GUID}
- HKLM\SOFTWARE\WOW6432Node\...
- HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\

## 关键字段
| 字段 | 含义 | 示例 |
| --- | --- | --- |
| DisplayName | 软件名 | "Visual Studio Code" |
| InstallLocation | 安装路径 | "C:\Users\xxx\AppData\Local\Programs\VSCode" |
| UninstallString | 卸载命令 | `""C:\...\unins000.exe""` |
| Publisher | 发布者 | "Microsoft Corporation" |
| DisplayVersion | 版本号 | "1.85.0" |
...
```

---

### T0.3 — PE 文件格式调研

- **优先级**：P0 🔴
- **复杂度**：中
- **依赖**：无
- **交付物**：
  - [x] 代码：`packforge-analyzer::dependencies` 中的 PE 解析（使用 goblin，已实现）
  - [ ] 文档：`docs/research/pe-format-analysis.md`（可选，代码已实现）

**调研内容**：

1. PE 导入表（Import Table）结构 — 如何提取 DLL 依赖
2. PE 资源段 — 提取版本信息、图标、公司名
3. 延迟加载导入（Delay Load Imports）
4. SxS Manifest（Side-by-Side Assembly）
5. 使用哪些 Rust crate 解析 PE（推荐 `goblin` 或 `object`）

---

### T0.4 — 安装框架特征指纹库

- **优先级**：P0 🟡
- **复杂度**：中
- **依赖**：无
- **交付物**：
  - [x] 代码：`packforge-analyzer::framework` 中的框架识别（已实现 NSIS/Inno/WiX/InstallShield/Electron 等）
  - [ ] 文档：`docs/research/install-framework-fingerprints.md`（可选，代码已实现）

**调研内容**：
每种安装框架的识别特征：

| 框架 | 卸载程序特征 | 文件特征 | 注册表特征 |
| --- | --- | --- | --- |
| NSIS | unins*.exe 含 "Nullsoft" | — | — |
| Inno Setup | unins000.exe + unins000.dat | — | — |
| WiX/MSI | MsiExec.exe /{GUID} | product.msi | HKLM\...\Products\{GUID} |
| InstallShield | setup.exe + setup.ini | data1.cab | — |
| Electron | uninstall.exe | resources/ | — |
| Advanced Installer | unins*.exe | — | — |

---

### T0.5 — 竞品分析

- **优先级**：P0 🟡
- **复杂度**：低
- **依赖**：无
- **交付物**：
  - [ ] 文档：`docs/research/competitive-analysis.md`

**分析对象**：

1. **dpkg-repack**（Linux）— 从已安装 .deb 重新打包
2. **PortableApps.com** — Windows 便携化标准
3. **Cameyo** — 应用虚拟化
4. **VMware ThinApp** — 应用打包
5. **Enigma Virtual Box** — 单文件打包
6. **BoxedApp Packer** — 虚拟化打包
7. **regshot** — 注册表快照对比工具

分析每个工具的：功能范围、技术方案、优缺点、我们可以借鉴什么。

---

## P1：便携化提取器（MVP）

### T1.1 — 软件发现模块实现

- **优先级**：P1 🔴
- **复杂度**：中
- **依赖**：T0.1, T0.2
- **交付物**：
  - [x] `packforge-scanner::discovery` 模块完整实现
  - [x] `packforge list` 命令可用
  - [x] 单元测试 + 集成测试

**实现内容**：

```rust
// 扫描三个注册表路径
// 解析每个软件条目的关键字段
// 返回结构化的软件列表
pub fn list_installed_software() -> Result<Vec<SoftwareInfo>>;

// 按名称/发布者模糊搜索
pub fn search_software(query: &str) -> Result<Vec<SoftwareInfo>>;
```

**验收标准**：

- 能列出系统中所有已安装软件
- 字段包含：名称、版本、发布者、安装路径、卸载命令
- 32/64 位软件都能发现
- 输出支持 JSON 和表格格式

---

### T1.2 — 安装目录扫描器

- **优先级**：P1 🔴
- **复杂度**：中
- **依赖**：T1.1
- **交付物**：
  - [x] `packforge-scanner::filesystem` 模块
  - [x] `packforge scan <软件名>` 命令可用
  - [x] 测试

**实现内容**：

```rust
// 递归扫描目录
pub fn scan_directory(path: &Path) -> Result<Vec<FileEntry>>;

// 分析每个文件的类型、大小、哈希
pub fn analyze_file(path: &Path) -> Result<FileEntry>;

// 检测 DLL 依赖（PE 导入表）
pub fn detect_dll_dependencies(path: &Path) -> Result<Vec<String>>;
```

**验收标准**：

- 能递归扫描安装目录下所有文件
- 每个文件记录：路径、大小、SHA-256、文件类型
- EXE/DLL 文件额外记录导入的 DLL 列表
- 输出 JSON 格式的扫描报告

---

### T1.3 — 文件分类器

- **优先级**：P1 🔴
- **复杂度**：中
- **依赖**：T0.3, T1.2
- **交付物**：
  - [x] `packforge-analyzer::classifier` 模块
  - [x] `packforge analyze <软件名>` 命令可用
  - [x] 测试

**分类规则**：

```rust
pub fn classify_file(entry: &FileEntry) -> FileCategory {
    // 1. 扩展名匹配
    // 2. 路径模式匹配（bin/, lib/, config/, docs/, temp/）
    // 3. PE 头分析（是 EXE/DLL 还是资源文件）
    // 4. 文件大小启发式（< 1KB 的可能是配置）
}

pub enum FileCategory {
    Core,           // 程序运行必需
    Library,        // 依赖库
    Plugin,         // 可选插件
    Config,         // 配置文件
    Documentation,  // 文档
    UserData,       // 用户数据
    Temporary,      // 临时文件
    Unknown,        // 需人工判断
}
```

---

### T1.4 — 便携化打包器

- **优先级**：P1 🔴
- **复杂度**：高
- **依赖**：T1.3
- **交付物**：
  - [x] `packforge-packager::portable` 模块
  - [x] `packforge extract <软件名>` 命令可用
  - [x] 生成的便携包可在新机器上运行（验证用例）

**实现内容**：

```rust
pub fn create_portable_package(
    profile: &SoftwareProfile,
    analysis: &AnalysisResult,
    output_dir: &Path,
    options: &ExtractOptions,
) -> Result<ExtractReport> {
    // 1. 创建输出目录结构
    // 2. 复制 Core + Library + Plugin 文件
    // 3. 复制默认 Config（排除用户数据）
    // 4. 生成 launcher.bat / launcher.sh
    // 5. 生成 manifest.json
    // 6. 可选：打包为 ZIP/7z
}

pub struct ExtractOptions {
    pub include_optional: bool,      // 是否包含可选组件
    pub include_docs: bool,          // 是否包含文档
    pub create_zip: bool,            // 是否打包为 ZIP
    pub launcher_type: LauncherType, // 启动脚本类型
}
```

**launcher.bat 模板**：

```batch
@echo off
setlocal
set "APP_DIR=%~dp0"
set "PATH=%APP_DIR%bin;%PATH%"
set "APPDATA=%APP_DIR%data"
start "" "%APP_DIR%bin\{main_exe}" %*
```

**验收标准**：

- 提取的便携包能在没有安装过该软件的 Windows 机器上运行
- 用户数据被正确剥离
- 启动脚本能正确设置环境变量
- 支持输出为 ZIP 压缩包

---

### T1.5 — CLI 框架 & 用户体验

- **优先级**：P1 🟡
- **复杂度**：中
- **依赖**：T1.1 ~ T1.4
- **交付物**：
  - [x] `packforge-cli` 完整实现
  - [x] 彩色输出、进度条
  - [x] 帮助文档
  - [x] 错误提示友好

**使用 `clap` crate 实现 CLI**：

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "packforge", about = "安装包提取器 & 安装程序生成器")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, default_value = "table")]
    format: OutputFormat, // table | json | yaml
    
    #[arg(short, long)]
    verbose: bool,
}
```

---

## P2：注册表 & 环境分析

### T2.1 — 注册表深度扫描

- **优先级**：P2 🔴
- **复杂度**：高
- **依赖**：T1.1
- **交付物**：
  - [x] `packforge-scanner::registry` 模块
  - [x] 能扫描指定软件相关的所有注册表条目

**实现策略**：

```rust
pub fn scan_registry_for_software(software: &SoftwareInfo) -> Vec<RegistryEntry> {
    // 策略 1：精确路径
    // 从 Uninstall 键获取的路径前缀匹配
    
    // 策略 2：名称搜索
    // 在 HKLM/HKCU 按 DisplayName 搜索
    
    // 策略 3：GUID 搜索
    // MSI 安装的软件有 ProductCode GUID
    
    // 策略 4：Publisher 搜索
    // 按发布者名搜索相关键
    
    // 策略 5：文件关联
    // .ext → progid → CLSID → 注册表链
}
```

---

### T2.2 — 系统快照 & Diff 引擎

- **优先级**：P2 🔴
- **复杂度**：高
- **依赖**：T2.1
- **交付物**：
  - [x] `packforge-scanner::snapshot` 模块（396 行，完整实现）
  - [x] `packforge snapshot` 命令（已接入库层）
  - [x] `packforge diff` 命令（已接入库层）

**实现内容**：

```rust
// 拍摄系统快照
pub fn take_snapshot() -> Result<SystemSnapshot> {
    // 1. 枚举所有注册表 Uninstall 条目
    // 2. 扫描 Program Files 目录
    // 3. 扫描已注册服务
    // 4. 扫描环境变量
    // 5. 记录时间戳
}

// 对比两次快照
pub fn diff_snapshots(before: &SystemSnapshot, after: &SystemSnapshot) -> SystemDiff {
    // 文件：新增 / 删除 / 修改
    // 注册表：新增 / 删除 / 修改
    // 服务：新增 / 删除
    // 环境变量：新增 / 修改
}
```

**这是最精确的分析方式**——先拍快照，装软件，再拍快照，对比。

---

### T2.3 — 服务 & COM 分析

- **优先级**：P2 🟡
- **复杂度**：高
- **依赖**：T2.1
- **交付物**：
  - [x] `packforge-scanner::services` 模块（含 COM 扫描）
  - [x] `packforge-scanner::com` 模块（合并到 services）

**实现内容**：

```rust
// 枚举 Windows 服务
pub fn scan_services() -> Vec<ServiceInfo>;
// 使用 Windows API: OpenSCManager, EnumServicesStatus

// 扫描 COM 组件注册
pub fn scan_com_registration() -> Vec<ComRegistration>;
// 扫描 HKCR\CLSID, HKCR\Interface, HKCR\TypeLib
```

---

### T2.4 — 依赖链解析器

- **优先级**：P2 🟡
- **复杂度**：高
- **依赖**：T0.3, T1.2
- **交付物**：
  - [x] `packforge-analyzer::dependencies` 模块

**实现内容**：

```rust
// 递归解析 PE 导入表，构建完整依赖树
pub fn resolve_full_dependency_tree(exe_path: &Path) -> Result<DepTree>;

// 过滤系统 DLL（维护一个已知系统 DLL 列表）
pub fn filter_system_dlls(deps: &mut Vec<Dependency>);

// 识别常见运行时
pub fn identify_runtimes(deps: &[Dependency]) -> Vec<RuntimeDependency>;
// VC++ 2005/2008/2010/2012/2013/2015-2022
// .NET Framework 2.0/3.5/4.x
// Java Runtime
// DirectX
// Visual Basic Runtime
```

---

### T2.5 — 可移植性评估器

- **优先级**：P2 🟡
- **复杂度**：中
- **依赖**：T2.1 ~ T2.4
- **交付物**：
  - [x] `packforge-analyzer::portability` 模块
  - [x] 可移植性评分报告

**评估维度**：

```rust
pub struct PortabilityReport {
    pub score: f32,                    // 0.0 ~ 1.0
    pub can_portable: bool,            // 是否可以便携化
    pub issues: Vec<PortabilityIssue>, // 问题列表
    pub suggestions: Vec<String>,      // 改进建议
}

// 评分规则：
// +0.3 无服务依赖
// +0.2 无 COM 注册
// +0.2 无 HKLM 写入
// +0.1 无硬编码路径
// +0.1 无驱动依赖
// +0.1 无全局互斥量
```

---

## P3：安装程序生成

### T3.1 — PackageManifest 构建器

- **优先级**：P3 🔴
- **复杂度**：中
- **依赖**：P2 完成
- **交付物**：
  - [x] `packforge-packager::manifest` 模块

**实现内容**：

```rust
// 从分析结果构建打包清单
pub fn build_manifest(
    profile: &SoftwareProfile,
    analysis: &AnalysisResult,
    options: &PackageOptions,
) -> Result<PackageManifest>;

// 清单包含：
// - 要打包的文件列表（映射源路径 → 安装路径）
// - 注册表条目（仅程序必需的）
// - 安装时要执行的动作（创建目录、注册 COM、创建服务等）
// - 卸载时要执行的清理动作
```

---

### T3.2 — Inno Setup 脚本生成器

- **优先级**：P3 🔴
- **复杂度**：高
- **依赖**：T3.1
- **交付物**：
  - [x] `packforge-packager::inno` 模块
  - [x] `packforge package --format inno` 命令
  - [x] 使用代码生成代替模板（handlebars 依赖可移除）

**Inno Setup 脚本结构**：

```ini
[Setup]
AppName={name}
AppVersion={version}
AppPublisher={publisher}
DefaultDirName={autopf}\{name}
DefaultGroupName={name}
OutputDir=output
OutputBaseFilename={name}-{version}-setup
Compression=lzma2
SolidCompression=yes
SetupIconFile={icon}

[Files]
; 从 manifest.entries 生成
Source: "staging\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
; 从 shortcuts 生成
Name: "{group}\{name}"; Filename: "{app}\{main_exe}"
Name: "{autodesktop}\{name}"; Filename: "{app}\{main_exe}"

[Registry]
; 从 manifest.registry 生成（仅程序必需的）
Root: HKLM; Subkey: "SOFTWARE\{name}"; ValueType: string; ValueName: "InstallPath"; ValueData: "{app}"

[Run]
; 安装后动作
Filename: "{app}\{main_exe}"; Description: "启动 {name}"; Flags: postinstall nowait

[UninstallRun]
; 卸载前动作

[UninstallDelete]
; 卸载时删除的文件/目录
Type: filesandordirs; Name: "{app}"
```

**编译集成**：

```rust
// 如果系统安装了 Inno Setup，直接调用编译
pub fn compile_inno_setup(iss_path: &Path) -> Result<PathBuf> {
    // 查找 iscc.exe 路径
    // 调用: iscc.exe /Q script.iss
    // 返回输出的 .exe 路径
}
```

---

### T3.3 — NSIS 脚本生成器（暂缓）

- **优先级**：P3 🟢
- **复杂度**：高
- **依赖**：T3.1
- **说明**：Inno Setup 覆盖 90% 场景，NSIS 后续按需实现
- **交付物**：
  - [ ] `packforge-packager::nsis` 模块
  - [ ] `packforge package --format nsis` 命令
  - [ ] 模板文件 `templates/nsis.nsi.hbs`

---

### T3.4 — WiX/MSI 项目生成器（暂缓）

- **优先级**：P3 🟢
- **复杂度**：高
- **依赖**：T3.1
- **说明**：MSI 格式复杂，后续按需实现
- **交付物**：
  - [ ] `packforge-packager::wix` 模块
  - [ ] `packforge package --format wix` 命令
  - [ ] 模板文件 `templates/wix.wxs.hbs`

---

### T3.5 — 打包流程编排

- **优先级**：P3 🔴
- **复杂度**：中
- **依赖**：T3.1 ~ T3.4
- **交付物**：
  - [x] `packforge package` 完整流程（Inno Setup 格式）
  - [ ] NSIS/WiX 格式支持（暂缓）

**已实现流程**：

```text
1. 扫描 & 分析软件 → AnalysisReport
2. 构建打包清单 → PackageManifest
3. 生成 Inno Setup 脚本 (.iss)
4. 编译安装程序（可选，需安装 Inno Setup）
5. 输出 .exe 安装程序
```

**CLI 命令**：

```bash
# 基本用法
packforge package "7-Zip"

# 完整参数
packforge package "7-Zip" --format inno --app-name "7-Zip" --publisher "Igor Pavlov"

# 只生成脚本，不编译
packforge package "7-Zip" --no-compile
```

---

## P4：智能净化 & 高级功能

### T4.1 — 智能用户数据剥离

- **优先级**：P4 🟡
- **复杂度**：高
- **依赖**：T1.3
- **交付物**：
  - [ ] `packforge-analyzer::purifier` 模块
  - [ ] 更精确的数据分类

**增强策略**：
1. 与已知安装包内容对比（如果有的话）
2. 文件修改时间分析（安装后修改的可能是用户数据）
3. 文件内容分析（搜索用户名、路径、个性化值）
4. 机器学习分类（长期目标）

---

### T4.2 — 安装框架逆向

- **优先级**：P4 🟡
- **复杂度**：极高
- **依赖**：T0.4
- **交付物**：
  - [ ] `packforge-analyzer::reverse` 模块

**思路**：
1. 识别原始安装框架（NSIS/Inno/WiX 等）
2. 尝试提取原始安装脚本（如 NSIS 脚本可以从 .exe 中提取部分）
3. 用提取的脚本作为模板重新打包

---

### T4.3 — 增量更新包生成

- **优先级**：P4 🟢
- **复杂度**：高
- **依赖**：P3
- **交付物**：
  - [ ] 对比两个版本的软件差异
  - [ ] 生成增量更新包（patch）

---

### T4.4 — 批量操作

- **优先级**：P4 🟢
- **复杂度**：中
- **依赖**：P3
- **交付物**：
  - [x] 批量提取多个软件 — `packforge batch-extract`
  - [x] 批量生成安装程序 — `packforge batch-package`
  - [x] 导出软件清单报告 — `packforge report`

**实现内容**：

```bash
# 导出软件清单（JSON/CSV）
packforge report --output software-list.json
packforge report --report-format csv --output software-list.csv

# 批量提取
packforge batch-extract "7-Zip" "Git" "Node.js" --output ./packages --zip
packforge batch-extract --list-file software-list.txt --output ./packages

# 批量打包
packforge batch-package "7-Zip" "Git" --output ./installers
packforge batch-package --list-file software-list.txt --no-compile
```

**安全改进**：
- 路径遍历防护（禁止 `..` 路径组件）
- 输出路径自动规范化

---

## P5：代码质量 & 工程改进

### T5.1 — 接入 snapshot/diff CLI 命令

- **优先级**：P5 🔴
- **复杂度**：中
- **依赖**：T2.2
- **交付物**：
  - [x] `cmd_snapshot` 接入 `packforge_scanner::snapshot::take_snapshot()`
  - [x] `cmd_diff` 接入 `packforge_scanner::snapshot::diff_snapshots()`
  - [x] JSON 序列化/反序列化快照文件
  - [x] 表格格式的差异报告输出

---

### T5.2 — 拆分 CLI main.rs

- **优先级**：P5 🟡
- **复杂度**：中
- **依赖**：无
- **交付物**：
  - [x] 将 1,663 行的 main.rs 拆分为独立模块
  - [x] 每个子命令一个文件

**实际拆分结果**：

```text
crates/packforge-cli/src/
├── main.rs          # CLI 定义 + 入口（~350 行）
├── utils.rs         # 进度条、彩色输出、时间格式、路径验证
└── commands/
    ├── mod.rs       # 模块导出
    ├── list.rs      # cmd_list
    ├── scan.rs      # cmd_scan
    ├── analyze.rs   # cmd_analyze
    ├── extract.rs   # cmd_extract
    ├── package.rs   # cmd_package
    ├── snapshot.rs  # cmd_snapshot
    ├── diff.rs      # cmd_diff
    ├── report.rs    # cmd_report + resolve_names
    └── batch.rs     # cmd_batch_extract + cmd_batch_package
```

---

### T5.3 — 清理未使用依赖

- **优先级**：P5 🟡
- **复杂度**：低
- **依赖**：无
- **交付物**：
  - [x] 移除 `pelite`（声明但未使用）
  - [x] 移除 `sevenz-rust`（声明但未使用）
  - [x] 移除 `petgraph`（声明但未使用）
  - [x] 移除 `handlebars`（使用代码生成代替模板）

---

### T5.4 — 补充 packforge-core 单元测试

- **优先级**：P5 🟡
- **复杂度**：低
- **依赖**：无
- **交付物**：
  - [x] 为核心数据结构添加序列化/反序列化测试（16 个新测试）
  - [x] 为 `ExtractOptions::Default` 添加测试
  - [x] 为 `SoftwareInfo` 构建和字段验证添加测试

---

### T5.5 — YAML 输出实现或移除

- **优先级**：P5 🟢
- **复杂度**：低
- **依赖**：无
- **交付物**：
  - [ ] 决定：实现 YAML 输出 或 从 CLI 选项中移除
  - [ ] 如移除，清理所有 `OutputFormat::Yaml` 分支

---

## 任务依赖图

```text
T0.1 ──┬── T1.1 ── T1.2 ── T1.3 ── T1.4 ── T1.5 (P1 MVP: 便携化提取)
       │                    ↑
T0.2 ──┤                    │
       │                    │
T0.3 ──┼────────────────────┘
       │
T0.4 ──┤
       │
T0.5 ──┤
       │
       ├── T2.1 ── T2.2 ──┬── T2.4 ── T2.5 (P2: 注册表 & 深度分析)
       │                   │
       ├── T2.3 ───────────┘
       │
       ├── T3.1 ── T3.2 (P3: Inno Setup 安装程序生成)
       │
       └── T3.3 / T3.4 / T3.5 / T4.x (后续按需)
```

## 执行建议

### 第一批（可并行）

- T0.1 项目搭建
- T0.2 注册表调研
- T0.3 PE 格式调研
- T0.4 安装框架调研
- T0.5 竞品分析

### 第二批（顺序执行，MVP）

- T1.1 → T1.2 → T1.3 → T1.4 → T1.5

### 第三批（可部分并行）

- T2.1 + T2.3 + T2.4（可并行）
- T2.2（依赖 T2.1）
- T2.5（依赖全部 P2）

### 第四批（顺序执行）

- T3.1 → T3.2（Inno Setup 生成）
- T3.3 / T3.4（NSIS/WiX，后续按需）
- T3.5（打包流程编排）

### 第五批（按需，合并自原 P4）

- T4.1 智能用户数据剥离（合并到 P2 分析阶段）
- T4.2 安装框架逆向（暂缓，优先级低）
- T4.3 增量更新包（暂缓）
- T4.4 批量操作（暂缓）
