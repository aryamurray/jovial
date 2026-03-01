use jovial_plugin::registry::PluginRegistry;

use jovial_plugin_java_collections::JavaCollectionsPlugin;

use crate::config::PluginRef;

/// Load builtin plugins into a registry, filtered by plugin refs.
///
/// Only plugins explicitly listed in `plugin_refs` (with `enabled != false`) are loaded.
/// If `plugin_refs` is empty, no plugins are registered and the default converter handles everything.
pub fn load_plugins(plugin_refs: &[PluginRef]) -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    if plugin_refs.is_empty() {
        return registry;
    }

    let all_plugins: Vec<Box<dyn jovial_plugin::traits::Plugin>> = vec![
        Box::new(JavaCollectionsPlugin),
    ];

    for plugin in all_plugins {
        let name = plugin.name().to_string();

        if let Some(pref) = plugin_refs.iter().find(|r| r.name == name) {
            if !pref.enabled {
                log::info!("plugin '{}' disabled by config", name);
                continue;
            }
            if pref.path.is_some() {
                log::warn!(
                    "plugin '{}': local plugin paths not yet supported, using builtin",
                    name
                );
            }
            registry.register(plugin);
        }
    }

    // Warn about unknown plugin refs
    let builtin_names: &[&str] = &[
        "java-collections",
    ];
    for pref in plugin_refs {
        if !builtin_names.contains(&pref.name.as_str()) {
            log::warn!("unknown plugin '{}' — not a builtin plugin", pref.name);
        }
    }

    registry
}
