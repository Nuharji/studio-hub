//! What Studio Hub offers over MCP (mcp.rs): which Studio apps are installed, starting one, the shared settings
//! (reading only). Installing and removing stay with the window.

use crate::{installed_apps, mcp, shared, Installed};
use serde_json::{json, Value};

pub fn run() { mcp::serve("studio-hub", env!("CARGO_PKG_VERSION"), &tools(), &call) }

fn tools() -> Vec<Value> {
    vec![
        mcp::tool("list_apps", "The Studio apps installed on this computer, with their versions.", json!({}), &[]),
        mcp::tool("open_app", "Starts an installed Studio app, or brings it to the front.", json!({"name": {"type": "string", "description": "e.g. Markdown Studio"}}), &["name"]),
        mcp::tool("shared_settings", "The settings all Studio apps share: theme, layout, language, work folder, sync folder.", json!({}), &[]),
    ]
}

fn studio_apps(list: Vec<Installed>) -> Vec<Installed> { list.into_iter().filter(|a| a.product.contains("Studio")).collect() }

/// What to start for the app of this name: never anything but an installed Studio app.
fn target(list: &[Installed], name: &str) -> Result<String, String> {
    let app = list.iter().find(|a| a.product.eq_ignore_ascii_case(name.trim())).ok_or_else(|| {
        format!("{name} is not installed; there are: {}", list.iter().map(|a| a.product.as_str()).collect::<Vec<_>>().join(", "))
    })?;
    app.exe.clone().or_else(|| Some(app.location.clone()).filter(|l| !l.is_empty())).ok_or_else(|| format!("{name}: no program found"))
}

fn call(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "list_apps" => Ok(json!(studio_apps(installed_apps()).iter().map(|a| json!({"name": a.product, "version": a.version})).collect::<Vec<_>>())),
        "open_app" => {
            let app = mcp::text(args, "name")?;
            crate::launch(target(&studio_apps(installed_apps()), &app)?)?;
            Ok(json!({"started": app}))
        }
        "shared_settings" => Ok(Value::Object(mcp::studio_dir().map(|d| shared::read_from(&d.join("shared.json"))).unwrap_or_default())),
        _ => Err(format!("unknown tool: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_only_studio_apps() {
        let app = |product: &str, exe: Option<&str>, location: &str| Installed { product: product.into(), exe: exe.map(str::to_string), location: location.into(), ..Default::default() };
        let list = studio_apps(vec![app("Plan Studio", Some("/a/plan"), "/a"), app("Slides Studio", None, "/Applications/Slides Studio.app"), app("Safari", Some("/s"), "/s"), app("Data Studio", None, "")]);
        assert_eq!(list.len(), 3, "other programs aren't shown");
        assert_eq!(target(&list, "plan studio").unwrap(), "/a/plan");
        assert_eq!(target(&list, "Slides Studio").unwrap(), "/Applications/Slides Studio.app");
        assert!(target(&list, "Data Studio").is_err());
        assert!(target(&list, "Safari").unwrap_err().contains("Plan Studio"));
    }
}
