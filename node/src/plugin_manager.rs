use std::{collections::HashMap, sync::Arc};

use crate::{node::NodeCommon, plugin::Plugin, types::PluginId};

#[derive(Default)]
pub struct PluginManager {
    plugins: HashMap<PluginId, Arc<dyn Plugin>>,
    name_to_id: HashMap<String, PluginId>,
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: HashMap::new(),
            name_to_id: HashMap::new(),
        }
    }

    pub fn register_plugin(&mut self, plugin: Arc<dyn Plugin>) -> Result<(), Arc<dyn Plugin>> {
        // Check if the plugin is already registered by ID or name
        let id = plugin.plugin_id();
        if self.plugins.contains_key(&id) {
            return Err(plugin);
        }
        let name = plugin.name().to_string();
        if self.name_to_id.contains_key(&name) {
            return Err(plugin);
        }

        // Register the plugin
        self.plugins.insert(id, plugin.clone());
        self.name_to_id.insert(name, id);
        Ok(())
    }

    pub fn get_plugin_by_id(&self, id: &PluginId) -> Option<&Arc<dyn Plugin>> {
        self.plugins.get(id)
    }

    pub fn get_plugin_by_name(&self, name: &str) -> Option<&Arc<dyn Plugin>> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.plugins.get(id))
    }

    pub fn generate_node_by_id(&self, id: &PluginId) -> Option<Arc<dyn NodeCommon>> {
        if let Some(plugin) = self.get_plugin_by_id(id) {
            Some(plugin.build())
        } else {
            None
        }
    }

    pub fn generate_node_by_name(&self, name: &str) -> Option<Arc<dyn NodeCommon>> {
        if let Some(plugin) = self.get_plugin_by_name(name) {
            Some(plugin.build())
        } else {
            None
        }
    }
}
