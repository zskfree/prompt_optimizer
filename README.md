<p align="center">
  <img src="./assets/readme/hero.svg" width="100%" alt="PromptOptimizer：在 Windows 中选中文字，通过全局热键优化提示词并复制结果">
</p>

<p align="center">
  <strong>一款轻量、无主窗口的 Windows 提示词优化工具。</strong><br>
  选中文字，按住 <kbd>Ctrl</kbd> 快速按两次 <kbd>F8</kbd>，优化结果自动进入剪贴板。
</p>

<p align="center">
  Windows 10/11 · Rust 2021 · OpenAI-compatible API · Portable
</p>

## 为什么使用 PromptOptimizer

PromptOptimizer 常驻系统托盘，把“选区 → 优化 → 粘贴”压缩成一次快捷操作。它不打开聊天窗口，也不维护会话历史；输入侧优先直接读取选区，仅在 UI Automation 失败时临时使用剪贴板兼容模式。

- **智能读取选区**：优先通过 Windows UI Automation 直接读取；失败时自动临时复制选区，读取后恢复原剪贴板，无需手动切换模式。
- **单轮无状态请求**：每次只发送本次选区和当前优化规则，使用任务 ID 隔离连续请求。
- **轻量常驻**：无打包式 GUI 框架、无异步运行时、无控制台窗口；当前 Release EXE 约 1.1 MiB，未打开设置窗口时后台常驻内存约 ~2 MB。
- **安静反馈**：在当前输入位置附近复用同一个小型状态框，显示“优化中…”和“已复制”。
- **兼容服务**：调用 OpenAI 兼容的 `/chat/completions` 接口，可配置地址、模型和采样参数。
- **内置设置 (v0.3.0)**：基于系统 WebView2 Runtime 渲染 Apple / Win11 风格设置界面，支持跟随系统深浅配色、平滑文字、自绘下拉框和胶囊 Toggle 开关；关闭窗口后会尽力清理本次设置页的临时数据目录，并统一右下角 Toast 的视觉。
- **绿色运行**：应用本体为单个 EXE，配置存放在 EXE 同目录；支持当前用户开机自启。设置窗口需要系统已安装 Microsoft Edge WebView2 Runtime。

## 快速开始

### 1. 准备程序

将 `PromptOptimizer.exe` 放入普通用户可写目录并运行。首次启动会在 EXE 同目录创建 UTF-8 编码的 `config.json`。

也可以从源码构建：

```powershell
cargo build --release
```

生成文件位于 `target\release\PromptOptimizer.exe`。

### 2. 配置模型

右键托盘图标，选择 **设置**。设置窗口分为“模型与服务”“优化规则”“应用行为”三页，可管理全部配置项。每套 API 配置可以保存多个模型（每行一个），并通过“当前模型”下拉框选择实际调用的模型。至少填写 API Key，并确认服务地址和当前模型可用；可先点击 **测试连接** 验证认证、地址和模型，再点击 **保存并应用**。测试连接会直接使用表单中当前选择的模型、温度和最大 Token 数，不再套用额外的测试参数。

连接失败时，窗口底部显示错误摘要，悬停可查看完整信息；配置中的 API Key 会自动隐藏。“优化规则”页只显示结果将复制到剪贴板，不再提供无实际选项的结果模式输入框。

设置会先完整校验，再统一写入 `config.json` 并立即生效。热键占用、开机自启或文件写入失败时，程序会保留原配置并在窗口底部显示原因。

“模型与服务”页可以管理多套命名配置。使用“当前配置”下拉框切换，点击 **新建** 创建配置，也可直接修改“配置名称”完成重命名。所有变更统一由窗口底部的 **保存并应用** 写入文件并立即生效，不再存在第二个“保存配置”步骤。

如需排障，也可以查看 EXE 同目录的 `config.json`，其结构如下：

```json
{
  "active_profile": "默认配置",
  "api_profiles": [
    {
      "name": "默认配置",
      "api_key": "YOUR_API_KEY",
      "base_url": "https://api.openai.com/v1",
      "models": ["gpt-4o-mini", "gpt-5-mini"],
      "model": "gpt-4o-mini",
      "temperature": 0.3,
      "max_tokens": 512
    }
  ],
  "hotkey": "Ctrl+DoubleF8",
  "system_prompt": "你是提示词优化助手……只返回优化后的提示词。",
  "result_mode": "clipboard",
  "play_sound": true,
  "auto_start": false
}
```

> `base_url` 应指向 API 根路径，例如 `https://api.openai.com/v1`；程序会自动追加 `/chat/completions`。v0.1.0 及更早版本的重复顶层 API 字段会在加载时自动迁移到当前配置，并在下次保存时清理。不要提交或分享包含真实 API Key 的配置文件。

### 3. 完成第一次优化

1. 在应用中选中一段提示词；Chrome 等 UI Automation 兼容性有限的应用会自动使用剪贴板兼容模式。
2. 按住 <kbd>Ctrl</kbd>，在约 0.52 秒间隔内快速按两次 <kbd>F8</kbd>。
3. 等待状态框从“优化中…”变为“已复制”。
4. 按 <kbd>Ctrl</kbd> + <kbd>V</kbd> 粘贴优化结果。

只按一次时，原本的 `Ctrl+F8` 会在短暂等待后正常传给当前应用。

## 工作方式

```text
当前焦点控件的选区
        │  Windows UI Automation；失败时临时复制并恢复剪贴板
        ▼
本次文本 + 配置中的优化规则
        │  独立、无状态的单轮请求
        ▼
OpenAI-compatible /chat/completions
        │  choices[0].message.content
        ▼
Unicode 文本写入剪贴板
```

程序只允许一个优化任务同时运行。网络请求由唯一工作线程处理，Win32 主线程继续响应托盘、热键和状态提示。

## 热键

默认值为：

```json
"hotkey": "Ctrl+DoubleF8"
```

支持以下格式：

- `Ctrl+DoubleF8`：默认值；按住 Ctrl，快速按两次 F8。
- `Ctrl+TripleF8`：按住 Ctrl，快速按三次 F8。
- 重复按键手势：支持 `Ctrl+Double...` / `Ctrl+Triple...` 加 `A–Z`、`0–9` 或 `F1–F24`。
- 普通组合键：修饰键 `Ctrl` / `Alt` / `Shift` / `Win` 加 `A–Z`、`0–9` 或 `F1–F24`。

普通组合键必须包含至少一个修饰键；`F12` 为系统保留键，不可使用。旧配置中的 `Ctrl+TripleA` 和 `Ctrl+DoubleA` 会在加载时自动迁移为 `Ctrl+DoubleF8`。

## 托盘菜单

- **设置**：打开 WebView2 设置窗口，直接查看、修改、校验并应用全部配置项。
- **重新加载配置**：用于加载外部工具手动修改过的 `config.json`；一般操作无需使用。
- **退出**：注销热键、移除托盘图标并结束程序。

Explorer 重启后，托盘图标会自动恢复；命名互斥量用于防止重复启动。

## 隐私与边界

- 输入侧优先不访问剪贴板；UI Automation 失败时会深拷贝可安全恢复的剪贴板数据、模拟 `Ctrl+C` 读取选区并立即恢复。兼容模式不再复用系统 OLE 剪贴板代理；遇到剪贴板占用或无法安全备份的格式时只提示本次失败，程序和快捷键会继续运行。输出侧只写入最终 Unicode 文本。
- 仅向配置的 API 服务发送当前选中文字、系统提示词和请求参数。
- API Key 及命名 API 配置以明文保存在本地 `config.json`，不会写入常规运行日志。
- 剪贴板管理器或 Windows 剪贴板历史可能记录兼容模式产生的临时复制；部分游戏、远程桌面和禁止复制的自绘控件仍可能无法读取选区。
- 当前只支持 Windows 和 `result_mode = "clipboard"`；不支持流式输出。

## 开发与验证

项目使用 `stable-x86_64-pc-windows-msvc` 工具链。提交前建议依次运行：

```powershell
cargo fmt --all -- --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
```

Release 配置启用了体积优化、LTO、单 codegen unit、`panic = "abort"` 和符号剥离。

## 项目结构

```text
PromptOptimizer/
├── assets/                 # 应用图标与 Windows 资源
│   └── readme/             # GitHub README 视觉资产
├── docs/
│   └── spec.md             # 初始开发规范
├── src/
│   ├── api.rs              # 无状态 API 请求与响应解析
│   ├── config.rs           # 配置生成、校验与恢复
│   ├── hotkey.rs           # 普通组合键和多击手势解析
│   └── windows_app/        # 设置窗口、UI Automation、剪贴板输出与自启
├── build.rs                # EXE 图标资源编译
└── Cargo.toml
```

详细设计背景见 [初始开发规范](./docs/spec.md)。该文档记录项目最初目标；当前行为以代码、测试和本 README 为准。
