use openidea_plugin::{
    ConsoleRule, Context, Error, Event, EventKind, Invocation, LogLevel, NotificationLevel, Plugin,
    host, register_plugin,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;

const SCHEMA: &str = include_str!("../settings.schema.json");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rule {
    enabled: bool,
    pattern: String,
    ignore_case: bool,
    foreground: String,
    background: String,
    style: String,
    whole_line: bool,
}

impl From<Rule> for ConsoleRule {
    fn from(rule: Rule) -> Self {
        Self {
            pattern: rule.pattern,
            enabled: rule.enabled,
            ignore_case: rule.ignore_case,
            foreground: (!rule.foreground.is_empty()).then_some(rule.foreground),
            background: (!rule.background.is_empty()).then_some(rule.background),
            styles: if rule.style.is_empty() {
                vec![]
            } else {
                vec![rule.style]
            },
            whole_line: rule.whole_line,
        }
    }
}

fn setting<T: DeserializeOwned>(key: &str) -> Result<T, Error> {
    let value = match host::setting(key) {
        Some(json) => serde_json::from_str(&json),
        None => serde_json::from_str::<Value>(SCHEMA)
            .map(|schema| schema["properties"][key]["default"].clone()),
    }
    .map_err(|error| Error::Invalid(format!("{key}: {error}")))?;
    serde_json::from_value(value).map_err(|error| Error::Invalid(format!("{key}: {error}")))
}

fn apply_rules() -> Result<(), Error> {
    let rules = if setting::<bool>("enabled")? {
        setting::<Vec<Rule>>("rules")?
            .into_iter()
            .map(ConsoleRule::from)
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    host::set_console_rules(&rules)
}

#[derive(Default)]
struct GrepConsole;

impl Plugin for GrepConsole {
    fn activate(&mut self, _ctx: &Context) -> Result<(), Error> {
        host::subscribe(EventKind::SettingsChanged)?;
        apply_rules()
    }

    fn run_command(
        &mut self,
        id: &str,
        _ctx: &Invocation,
        _arg: Option<&str>,
    ) -> Result<Option<String>, Error> {
        match id {
            "reload" => {
                apply_rules()?;
                Ok(Some("Console highlight rules reloaded.".into()))
            }
            _ => Err(Error::NotFound(format!("command {id}"))),
        }
    }

    fn handle_events(&mut self, events: &[Event], _ctx: &Invocation) {
        if events
            .iter()
            .any(|event| matches!(event, Event::Other { kind, .. } if kind == "settings-changed"))
            && let Err(error) = apply_rules()
        {
            let message = format!("Could not apply console rules; previous rules kept. {error}");
            host::log(LogLevel::Error, &message);
            host::notify(NotificationLevel::Error, "Grep Console", &message);
        }
    }
}

register_plugin!(GrepConsole);

#[cfg(test)]
mod tests {
    use super::*;
    use openidea_plugin::mock::MockHost;
    use serde_json::json;

    fn changed(plugin: &mut GrepConsole, host: &MockHost) {
        plugin.handle_events(
            &[Event::Other {
                kind: "settings-changed".into(),
                data: r#"["rules"]"#.into(),
            }],
            &host.invocation(),
        );
    }

    #[test]
    fn activates_with_schema_defaults_and_applies_setting_changes() {
        let host = MockHost::install().with_capabilities(&["events"]);
        let mut plugin = GrepConsole;
        plugin
            .activate(&host.context("openidea.grep-console"))
            .unwrap();
        let defaults = host.console_rules();
        assert_eq!(defaults.len(), 3);
        assert_eq!(
            defaults[0],
            ConsoleRule::new(r"\b(ERROR|FATAL)\b")
                .ignore_case()
                .foreground("bright-red")
                .style("bold")
                .whole_line()
        );
        assert!(!defaults[2].whole_line);

        host.set_setting(
            "rules",
            &json!([{
                "enabled": false, "pattern": "JOB-\\d+", "ignoreCase": true,
                "foreground": "", "background": "blue", "style": "underline", "wholeLine": false
            }])
            .to_string(),
        );
        changed(&mut plugin, &host);
        assert_eq!(
            host.console_rules(),
            vec![
                ConsoleRule::new(r"JOB-\d+")
                    .enabled(false)
                    .ignore_case()
                    .background("blue")
                    .style("underline")
            ]
        );
    }

    #[test]
    fn disabling_clears_rules_and_reenabling_restores_them() {
        let host = MockHost::install();
        let mut plugin = GrepConsole;
        apply_rules().unwrap();
        let rules = host.console_rules();
        host.set_setting("enabled", "false");
        changed(&mut plugin, &host);
        assert!(host.console_rules().is_empty());
        host.set_setting("enabled", "true");
        plugin
            .run_command("reload", &host.invocation(), None)
            .unwrap();
        assert_eq!(host.console_rules(), rules);
        host.set_setting("rules", "[]");
        changed(&mut plugin, &host);
        assert!(host.console_rules().is_empty());
    }

    #[test]
    fn invalid_settings_keep_previous_rules_and_report_errors() {
        let host = MockHost::install();
        let mut plugin = GrepConsole;
        apply_rules().unwrap();
        let previous = host.console_rules();
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let rule = &schema["properties"]["rules"]["default"][0];
        let mut long_pattern = rule.clone();
        long_pattern["pattern"] = json!("x".repeat(513));
        for invalid in [
            "not json".to_owned(),
            json!([{"pattern": "missing fields"}]).to_string(),
            json!([long_pattern]).to_string(),
            serde_json::to_string(&vec![rule; 101]).unwrap(),
        ] {
            host.set_setting("rules", &invalid);
            changed(&mut plugin, &host);
            assert_eq!(host.console_rules(), previous);
            assert!(
                plugin
                    .run_command("reload", &host.invocation(), None)
                    .is_err()
            );
        }
        assert_eq!(host.notifications().len(), 4);
        assert_eq!(host.logs().len(), 4);
    }
}
