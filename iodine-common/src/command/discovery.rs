use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiscoveryCommand {
    InitRegistries,
    ReloadRegistries,
    ReloadRegistry { registry_id: Uuid },
    RemoveTrackedRegistry { registry_id: Uuid },
}

