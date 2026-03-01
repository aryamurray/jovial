use std::collections::HashMap;

use jovial_plugin::registry::PluginRegistry;
use jovial_plugin::traits::Plugin;

use jovial_plugin_java_collections::JavaCollectionsPlugin;
use jovial_plugin_java_io::JavaIoPlugin;
use jovial_plugin_java_strings::JavaStringsPlugin;

use crate::config::PluginRef;

/// Load builtin plugins into a registry in config order.
///
/// Iterates `plugin_refs` in order, looking up each in a builtin HashMap.
/// Only enabled plugins are registered. Unknown names emit a warning.
/// If `plugin_refs` is empty, no plugins are registered and the default converter handles everything.
pub fn load_plugins(plugin_refs: &[PluginRef]) -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    if plugin_refs.is_empty() {
        return registry;
    }

    let mut builtins: HashMap<&str, Box<dyn Plugin>> = HashMap::new();
    builtins.insert("java-collections", Box::new(JavaCollectionsPlugin));
    builtins.insert("java-strings", Box::new(JavaStringsPlugin));
    builtins.insert("java-io", Box::new(JavaIoPlugin));

    for pref in plugin_refs {
        if !pref.enabled {
            log::info!("plugin '{}' disabled by config", pref.name);
            continue;
        }
        if pref.path.is_some() {
            log::warn!(
                "plugin '{}': local plugin paths not yet supported, using builtin",
                pref.name
            );
        }
        if let Some(plugin) = builtins.remove(pref.name.as_str()) {
            registry.register(plugin);
        } else {
            log::warn!("unknown plugin '{}'", pref.name);
        }
    }

    registry
}
