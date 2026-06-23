# PackForge — 安装包提取器 & 安装程序生成器

从已安装软件的状态出发，逆向生成可分发的安装程序或便携化包。
解决"装完就删，要用时又得重新找"的痛点。

> **定位**：免费开源、自动化、Rust 实现。不依赖快照即可运行（也支持快照精确模式）。

## 核心功能

```bash
# 列出已安装软件
packforge list
packforge list --query "7-Zip"

# 深度扫描软件
packforge scan "7-Zip"
packforge scan "7-Zip" --hash --pe

# 分析可移植性
packforge analyze "7-Zip"

# 提取为便携化包
packforge extract "7-Zip" --output ./packages --zip

# 生成安装程序（Inno Setup）
packforge package "7-Zip"
packforge package "7-Zip" --app-name "My 7-Zip" --publisher "Custom"

# 批量操作
packforge report --output software-list.json
packforge batch-extract "7-Zip" "Git" "Node.js" --output ./packages
packforge batch-package "7-Zip" "Git" --no-compile --output ./installers
```

## 技术栈

- **语言**：Rust 2024 Edition
- **构建**：Cargo workspace（5 个 crate）
- **Windows API**：`windows-registry` crate
- **打包输出**：Inno Setup 脚本（.iss）→ .exe 安装程序
- **测试**：83 个单元测试，clippy 零警告

## 关键依赖

| 用途 | Crate |
| --- | --- |
| 注册表操作 | `windows-registry` |
| PE 解析 | `goblin` |
| CLI | `clap` (ValueEnum) |
| 进度条 | `indicatif` |
| 彩色输出 | `colored` |
| 并行扫描 | `rayon` |
| 哈希 | `sha2` |
| 压缩 | `zip` |
| CSV 导出 | `csv` |
| 序列化 | `serde` + `serde_json` |
| 路径处理 | `camino` |

## 目录结构

```text
packforge/
├── Cargo.toml                   # Workspace 配置
├── README.md
├── TASKS.md
├── crates/
│   ├── packforge-core/          # 核心数据模型（SoftwareProfile, FileEntry, PackageManifest）
│   ├── packforge-scanner/       # 文件系统 & 注册表扫描（discovery, filesystem, registry, services, snapshot）
│   ├── packforge-analyzer/      # 智能分析（分类、依赖、可移植性、运行时检测）
│   ├── packforge-packager/      # 打包输出（portable, manifest, inno）
│   └── packforge-cli/           # CLI 入口（10 个子命令）
└── tests/                       # 集成测试
```

## CLI 命令

| 命令 | 说明 | 示例 |
|------|------|------|
| `list` | 列出已安装软件 | `packforge list --query "7-Zip"` |
| `scan` | 深度扫描软件 | `packforge scan "7-Zip" --hash` |
| `analyze` | 分析可移植性 | `packforge analyze "Git"` |
| `extract` | 提取为便携包 | `packforge extract "7-Zip" --zip` |
| `package` | 生成安装程序 | `packforge package "7-Zip"` |
| `snapshot` | 拍摄系统快照 | `packforge snapshot --output before.json` |
| `diff` | 对比快照差异 | `packforge diff before.json after.json` |
| `report` | 导出软件清单 | `packforge report --format csv` |
| `batch-extract` | 批量提取 | `packforge batch-extract "7-Zip" "Git"` |
| `batch-package` | 批量打包 | `packforge batch-package "7-Zip" --no-compile` |

## 开发状态

| 阶段 | 任务 | 状态 |
|------|------|------|
| P0 | 项目搭建 | ✅ 完成 |
| P1 | T1.1~T1.5 MVP（便携化提取） | ✅ 完成 |
| P2 | T2.1~T2.5 深度分析 | ✅ 完成 |
| P3 | T3.1~T3.5 安装程序生成 | ✅ Inno Setup 完成，NSIS/WiX 暂缓 |
| P4 | T4.4 批量操作 | ✅ 完成 |
| P5 | T5.1~T5.4 代码质量改进 | ✅ 完成 |

**测试统计**：83 个测试全部通过，clippy 零警告

### 待完成

- [ ] NSIS/WiX 格式支持（按需）

## 快速开始

### 安装

```bash
# 从源码构建
git clone https://github.com/packforge/packforge.git
cd packforge
cargo build --release

# 二进制文件位于 target/release/packforge.exe
```

### 使用示例

```bash
# 1. 列出所有已安装软件
packforge list

# 2. 搜索特定软件
packforge list --query "Visual"

# 3. 分析软件可移植性
packforge analyze "7-Zip"

# 4. 提取为便携化包
packforge extract "7-Zip" --output ./my-packages --zip

# 5. 生成 Inno Setup 安装程序
packforge package "7-Zip" --app-name "7-Zip Portable"

# 6. 导出软件清单
packforge report --output software-list.csv --report-format csv

# 7. 批量打包
packforge batch-package "7-Zip" "Git" "Node.js" --output ./installers
```

## 许可证

MIT OR Apache-2.0
