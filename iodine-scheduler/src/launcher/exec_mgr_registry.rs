use std::{collections::HashMap, sync::Arc};

use iodine_common::resource_manager::{ExecutionContextKind, ExecutionManager};
use uuid::Uuid;

#[derive(Debug)]
pub struct ExecutionManagerRegistry {
    managers: HashMap<Uuid, Arc<dyn ExecutionManager>>,
}

impl ExecutionManagerRegistry {
    pub fn new() -> Self {
        Self {
            managers: HashMap::new(),
        }
    }

    pub fn register_manager(&mut self, manager: Arc<dyn ExecutionManager>) {
        let manager_id = manager.manager_id();
        self.managers.insert(manager_id, manager);
    }

    pub fn get_manager_by_id(&self, manager_id: Uuid) -> Option<Arc<dyn ExecutionManager>> {
        self.managers.get(&manager_id).cloned()
    }

    pub fn get_manager(&self, kind: ExecutionContextKind) -> Option<Arc<dyn ExecutionManager>> {
        self.managers
            .values()
            .find(|manager| manager.supported_resource_type() == kind)
            .cloned()
    }
}
