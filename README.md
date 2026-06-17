# EveryAPI for Zed

在 Zed 里用 EveryAPI 网关：一把 key、240+ 模型（Claude / GPT / Gemini / DeepSeek…）。

完整的接入分两半，对应 Zed 的两套机制：

| 能力 | 实现方式 | 需要本扩展？ |
| --- | --- | --- |
| Agent Panel / inline assist 用 EveryAPI 模型 | `settings.json` 的 `language_models.openai_compatible`（Zed 内置） | 否 |
| Agent 里查余额 / 用量 / 模型目录 | 本扩展（MCP context server，启动 [`@everyapi-ai/mcp`](../../packages/mcp)） | 是 |

> Zed 的扩展 API 没有 LLM provider 扩展点，模型接入只能走 settings —— 这不是省事，是 Zed 的设计。

## 1. 模型接入（settings.json）

`⌘,` 打开 settings.json，加：

```jsonc
{
  "language_models": {
    "openai_compatible": {
      "EveryAPI": {
        "api_url": "https://api.everyapi.ai/v1",
        "available_models": [
          { "name": "claude-3-5-sonnet", "max_tokens": 200000 },
          { "name": "gpt-4o", "max_tokens": 128000 },
          { "name": "gemini-2.5-pro", "max_tokens": 1000000 },
          { "name": "deepseek-chat", "max_tokens": 64000 }
        ]
      }
    }
  },
  "agent": {
    "default_model": { "provider": "EveryAPI", "model": "claude-3-5-sonnet" }
  }
}
```

API key 不进配置文件：Agent Panel → 设置 → EveryAPI provider 里粘贴 `sk-everyapi-...`，Zed 存系统 keychain。配好后 Agent Panel、inline assist（`ctrl-enter`）的模型选择器里就有上面这些模型。

## 2. 账户工具（本扩展）

装好扩展后，Agent Panel → Settings 找到 **EveryAPI** context server，填 `api_key`。扩展会通过 Zed 自带的 Node 运行时安装并启动 `@everyapi-ai/mcp`，agent 获得三个只读工具：

- `get_wallet` —— "我的 everyapi 余额还剩多少？"
- `get_usage_summary` —— "今天花了多少？哪个模型用得最多？"
- `list_models` —— "网关里有哪些 claude 模型？"

## 开发

```sh
# 本地调试：Zed 命令面板 → "zed: install dev extension" → 选 apps/zed 目录
# Zed 会用 wasm32-wasip2 工具链就地编译

# 仅类型/借用检查：
cargo check --manifest-path apps/zed/Cargo.toml
```

发布前置条件：`@everyapi-ai/mcp` 需先发布到 npm（扩展运行时按 latest 版本安装它）。扩展本体通过 [zed-industries/extensions](https://github.com/zed-industries/extensions) 仓库提交，`extensions.toml` 条目用 `path = "apps/zed"` 指向本目录。
