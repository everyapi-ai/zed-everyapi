// EveryAPI Zed extension.
//
// What this is, in one sentence: an MCP context-server extension that gives
// Zed's Agent Panel read-only access to the EveryAPI account — balance,
// usage, and the 240+ model catalog — by launching `@everyapi-ai/mcp`
// through Zed's bundled Node runtime.
//
// What this is NOT: a language model provider. Zed's extension API has no
// LLM-provider extension point; routing the Agent Panel / inline assist
// through the EveryAPI gateway is done in settings.json via
// `language_models.openai_compatible` (see README.md), no extension needed.

use zed_extension_api::{
    self as zed, serde_json, settings::ContextServerSettings, Command, ContextServerConfiguration,
    ContextServerId, Project, Result,
};

const PACKAGE_NAME: &str = "@everyapi-ai/mcp";
// `vp pack` (tsdown) bundles the server into a single ESM file, so we launch
// it by path through Zed's node binary instead of resolving the package's bin
// shim (node_modules/.bin symlinks are not created reliably across npm
// versions). Keep this filename in lockstep with packages/mcp `vp pack`
// output — it emits dist/index.mjs (.mjs, not .js).
const SERVER_PATH: &str = "node_modules/@everyapi-ai/mcp/dist/index.mjs";

const CONTEXT_SERVER_ID: &str = "everyapi";

// Pin the MCP server version rather than tracking `@latest`: a future breaking
// release of @everyapi-ai/mcp should not auto-propagate to installed
// extensions. Bump this in lockstep with packages/mcp/package.json.
const REQUIRED_MCP_VERSION: &str = "0.1.2";

const API_KEY_PLACEHOLDER: &str = "sk-everyapi-...";

/// Accept a user-supplied key only when it's non-empty and not the
/// default-settings placeholder, so saving the pre-filled template doesn't
/// launch the server with a fake key.
fn resolve_api_key(raw: Option<&str>) -> Option<String> {
    raw.filter(|key| !key.is_empty() && *key != API_KEY_PLACEHOLDER)
        .map(str::to_string)
}

/// Build the child process env: the key is always present; the base URL is
/// only forwarded when the user set a non-default one.
fn build_env(api_key: String, base_url: Option<String>) -> Vec<(String, String)> {
    let mut env = vec![("EVERYAPI_API_KEY".to_string(), api_key)];
    if let Some(base_url) = base_url {
        env.push(("EVERYAPI_BASE_URL".to_string(), base_url));
    }
    env
}

struct EveryApiExtension;

impl zed::Extension for EveryApiExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Command> {
        let settings = ContextServerSettings::for_project(CONTEXT_SERVER_ID, project)?;
        let Some(settings) = settings.settings else {
            return Err(
                "missing settings — open Agent Panel → Settings → EveryAPI and set `api_key`"
                    .into(),
            );
        };
        let api_key = resolve_api_key(settings.get("api_key").and_then(serde_json::Value::as_str))
            .ok_or_else(|| {
                "missing `api_key` in EveryAPI context server settings — get one at https://app.everyapi.ai → API Keys".to_string()
            })?;
        let base_url = settings
            .get("base_url")
            .and_then(serde_json::Value::as_str)
            .filter(|url| !url.is_empty())
            .map(str::to_string);

        // Install the pinned version when it's not already present. Stay usable
        // offline: if the install fails but some copy is already installed, we
        // launch that and upgrade on a later (online) restart.
        let installed = zed::npm_package_installed_version(PACKAGE_NAME)?;
        if installed.as_deref() != Some(REQUIRED_MCP_VERSION) {
            if let Err(err) = zed::npm_install_package(PACKAGE_NAME, REQUIRED_MCP_VERSION) {
                if installed.is_none() {
                    return Err(format!(
                        "failed to install {PACKAGE_NAME}@{REQUIRED_MCP_VERSION} from npm: {err}"
                    ));
                }
            }
        }

        let server_path = std::env::current_dir()
            .map_err(|err| format!("failed to resolve extension work dir: {err}"))?
            .join(SERVER_PATH)
            .to_string_lossy()
            .to_string();

        Ok(Command {
            command: zed::node_binary_path()?,
            args: vec![server_path],
            env: build_env(api_key, base_url),
        })
    }

    fn context_server_configuration(
        &mut self,
        _context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<Option<ContextServerConfiguration>> {
        Ok(Some(ContextServerConfiguration {
            installation_instructions: include_str!("../configuration/installation_instructions.md")
                .to_string(),
            default_settings: include_str!("../configuration/default_settings.jsonc").to_string(),
            settings_schema: include_str!("../configuration/settings_schema.json").to_string(),
        }))
    }
}

zed::register_extension!(EveryApiExtension);

#[cfg(test)]
mod tests {
    use super::{build_env, resolve_api_key};

    #[test]
    fn rejects_empty_and_placeholder_keys() {
        assert_eq!(resolve_api_key(None), None);
        assert_eq!(resolve_api_key(Some("")), None);
        assert_eq!(resolve_api_key(Some("sk-everyapi-...")), None);
    }

    #[test]
    fn accepts_a_real_key() {
        assert_eq!(
            resolve_api_key(Some("sk-everyapi-abc123")),
            Some("sk-everyapi-abc123".to_string())
        );
    }

    #[test]
    fn env_has_key_only_without_base_url() {
        assert_eq!(
            build_env("k".to_string(), None),
            vec![("EVERYAPI_API_KEY".to_string(), "k".to_string())]
        );
    }

    #[test]
    fn env_appends_base_url_when_present() {
        assert_eq!(
            build_env("k".to_string(), Some("https://gw/v1".to_string())),
            vec![
                ("EVERYAPI_API_KEY".to_string(), "k".to_string()),
                ("EVERYAPI_BASE_URL".to_string(), "https://gw/v1".to_string()),
            ]
        );
    }
}
