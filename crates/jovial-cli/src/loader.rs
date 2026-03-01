use jovial_plugin::registry::PluginRegistry;

use jovial_plugin_spring_web::SpringWebPlugin;
use jovial_plugin_spring_data::SpringDataPlugin;
use jovial_plugin_spring_tx::SpringTxPlugin;
use jovial_plugin_guava::GuavaPlugin;
use jovial_plugin_jackson::JacksonPlugin;
use jovial_plugin_lombok::LombokPlugin;
use jovial_plugin_slf4j::Slf4jPlugin;

/// Load builtin plugins into a registry, filtered by the enabled list.
pub fn load_plugins(enabled: &[String]) -> PluginRegistry {
    let mut registry = PluginRegistry::new();

    let all_plugins: Vec<Box<dyn jovial_plugin::traits::Plugin>> = vec![
        Box::new(LombokPlugin),
        Box::new(SpringWebPlugin),
        Box::new(SpringDataPlugin),
        Box::new(SpringTxPlugin),
        Box::new(JacksonPlugin),
        Box::new(GuavaPlugin),
        Box::new(Slf4jPlugin),
    ];

    for plugin in all_plugins {
        if enabled.is_empty() || enabled.contains(&plugin.name().to_string()) {
            registry.register(plugin);
        }
    }

    registry
}
