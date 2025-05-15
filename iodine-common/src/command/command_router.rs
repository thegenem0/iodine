use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    sync::Arc,
};

use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::error::Error;

#[derive(Eq, PartialEq, Hash, Clone, Debug, Copy)]
/// Represents a routing key for a command router.
/// ---
/// The routing key is used to determine the appropriate
/// command type / handler to use.
enum RoutingKey {
    /// Represents a singleton service.
    /// ---
    /// Since there is only one of this service type,
    /// determining the appropriate command type / handler
    /// can be done using the `TypeId` only.
    Singleton(TypeId),

    /// Represents an instanced service.
    /// ---
    /// Since there may be multiple instances of this service type,
    /// determining the appropriate command type / handler
    /// requires the `TypeId` and the `Uuid` of the instance.
    Instanced(TypeId, Uuid),
}

#[derive(Clone)]
/// Command router
/// ---
/// It handles dispatching of commands to their appropriate handlers,
/// by abstracting the tedious task of creating communication channels
/// for services and keeping track of passing senders around.
pub struct CommandRouter {
    /// Stores `Box<dyn Any + Send + Sync>` which are
    /// downcast to specific `mpsc::Sender<CommandType>`
    senders: Arc<Mutex<HashMap<RoutingKey, Box<dyn Any + Send + Sync>>>>,

    /// Stores `Box<dyn Any + Send + Sync>` which are
    /// downcast to specific `mpsc::Receiver<CommandType>`
    receivers: Arc<Mutex<HashMap<RoutingKey, Box<dyn Any + Send + Sync>>>>,

    /// This is mostly for debugging and for better error messages.
    /// Stores the keys of the receivers that are currently subscribed,
    /// and can no longer be subscribed to.
    subscribed_receivers: Arc<Mutex<HashSet<RoutingKey>>>,

    /// Buffer size for singleton and instanced channels.
    singleton_channel_buf: usize,

    /// Buffer size for instanced channels.
    instanced_channel_buf: usize,
}

impl CommandRouter {
    /// Creates a new `CommandRouter`.
    /// Optionally, the buffer size for singleton and instanced channels can be specified.
    /// If not specified, the defaults are:
    /// *5* for singleton channels,
    /// *20* for instanced channels.
    pub fn new(singleton_channel_buf: Option<usize>, instanced_channel_buf: Option<usize>) -> Self {
        let singleton_channel_buf = singleton_channel_buf.unwrap_or(5);
        let instanced_channel_buf = instanced_channel_buf.unwrap_or(20);

        Self {
            senders: Arc::new(Mutex::new(HashMap::new())),
            receivers: Arc::new(Mutex::new(HashMap::new())),
            subscribed_receivers: Arc::new(Mutex::new(HashSet::new())),
            singleton_channel_buf,
            instanced_channel_buf,
        }
    }

    /// Registers a handler for a singleton service that processes commands of type `C`.
    /// `C` is typically an enum like `CoordinatorCommand`.
    /// Returns the receiver for the channel.
    /// The sender is kept internally, and is used in the `dispatch` method
    /// to send commands to the registered handler.
    pub async fn register_singleton_handler<C: Any + Send + 'static>(&self) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<C>(self.singleton_channel_buf);
        let key = RoutingKey::Singleton(TypeId::of::<C>());
        {
            let mut senders_map = self.senders.lock().await;

            if senders_map.contains_key(&key) {
                let error_msg = format!(
                    "Singleton service for type {:?} (key {:?}) already registered.",
                    std::any::type_name::<C>(),
                    key
                );
                println!("Warning: {}", error_msg);
                return Err(Error::Internal(error_msg));
            }

            senders_map.insert(key, Box::new(tx));
        }
        {
            let mut receivers_map = self.receivers.lock().await;
            if receivers_map.contains_key(&key) {
                let error_msg = format!(
                    "Singleton service for type {:?} (key {:?}) already registered.",
                    std::any::type_name::<C>(),
                    key
                );
                println!("Warning: {}", error_msg);
                return Err(Error::Internal(error_msg));
            }
            receivers_map.insert(key, Box::new(rx));
        }

        Ok(())
    }

    /// Registers a handler for a specific instance of a
    /// given service that processes commands of type `C`.
    /// The instance is identified by `instance_id`.
    /// This instance processes commands of type `C` (e.g., `LauncherCommand`).
    /// Returns the receiver for the channel.
    /// The sender is kept internally, and is used in the `dispatch` method
    /// to send commands to the registered handler.
    pub async fn register_instanced_handler<C: Any + Send + 'static>(
        &self,
        instance_id: Uuid,
    ) -> Result<(), Error> {
        let (tx, rx) = mpsc::channel::<C>(self.instanced_channel_buf);
        let key = RoutingKey::Instanced(TypeId::of::<C>(), instance_id);
        {
            let mut senders_map = self.senders.lock().await;

            if senders_map.contains_key(&key) {
                let error_msg = format!(
                    "Instance service for ID {} (key {:?}) already registered.",
                    instance_id, key
                );
                println!("Warning: {}", error_msg);
                return Err(Error::Internal(error_msg));
            }

            senders_map.insert(key, Box::new(tx));
        }
        {
            let mut receivers_map = self.receivers.lock().await;
            if receivers_map.contains_key(&key) {
                let error_msg = format!(
                    "Instance service for ID {} (key {:?}) already registered.",
                    instance_id, key
                );
                println!("Warning: {}", error_msg);
                return Err(Error::Internal(error_msg));
            }
            receivers_map.insert(key, Box::new(rx));
        }

        Ok(())
    }

    /// Deregisters a handler for a singleton service,
    /// which processes commands of type `C`.
    pub async fn deregister_singleton_handler<C: Any + Send + 'static>(&self) {
        let key = RoutingKey::Singleton(TypeId::of::<C>());
        {
            let mut senders_map = self.senders.lock().await;

            if senders_map.remove(&key).is_some() {
                println!("Successfully deregistered singleton sender for {:?}", key);
            }
        }
        {
            let mut receivers_map = self.receivers.lock().await;
            if receivers_map.remove(&key).is_some() {
                println!("Successfully deregistered singleton receiver for {:?}", key);
            }
        }
    }

    /// Deregisters a handler for a specific instance of a
    /// given service, which processes commands of type `C`.
    /// The instance is identified by `instance_id`.
    pub async fn deregister_instanced_handler<C: Any + Send + 'static>(&self, instance_id: Uuid) {
        let key = RoutingKey::Instanced(TypeId::of::<C>(), instance_id);
        {
            let mut senders_map = self.senders.lock().await;

            if senders_map.remove(&key).is_some() {
                println!("Successfully deregistered instanced sender for {:?}", key);
            }
        }
        {
            let mut receivers_map = self.receivers.lock().await;
            if receivers_map.remove(&key).is_some() {
                println!("Successfully deregistered instanced receiver for {:?}", key);
            }
        }
    }

    /// Dispatches a command of type `C` to its registered handler.
    /// The command `C` itself is sent through the channel.
    pub async fn dispatch<C: Any + Send + 'static>(
        &self,
        command: C,
        target_instance_id: Option<Uuid>,
    ) -> Result<(), Error> {
        let (key, target_descriptor) = Self::get_routing_key::<C>(self, target_instance_id)?;

        let sender_for_dispatch = {
            let handlers_map = self.senders.lock().await;
            handlers_map
                .get(&key)
                .and_then(|boxed_sender| boxed_sender.downcast_ref::<mpsc::Sender<C>>())
                .cloned()
        };

        if let Some(sender) = sender_for_dispatch {
            sender.send(command).await.map_err(|e| {
                Error::Internal(format!(
                    "CommandRouter: Failed to send command to {}: {}",
                    target_descriptor, e
                ))
            })
        } else {
            Err(Error::Internal(format!(
                "CommandRouter: No handler registered for command type {} (routing key: {:?})",
                target_descriptor, key
            )))
        }
    }

    /// Subscribes to a singleton service that processes commands of type `C`.
    /// `C` is typically an enum like `CoordinatorCommand`.
    /// Returns the receiver for the channel.
    ///
    /// **Warning**: `mpsc::Receiver` channels are not `Clone`,
    /// as they are meant to be used in one single place,
    /// hence the name, multi-producer-single-consumer.
    /// This method removes the receiver from the internal map
    /// of the CommandRouter, and returns it.
    /// The caller is responsible for keeping the receiver alive
    /// until it is no longer needed.
    pub async fn subscribe<C: Any + Send + 'static>(
        &self,
        target_instance_id: Option<Uuid>,
    ) -> Result<mpsc::Receiver<C>, Error> {
        let (key, target_descriptor) = Self::get_routing_key::<C>(self, target_instance_id)?;

        let mut subscribed_receivers = self.subscribed_receivers.lock().await;
        if subscribed_receivers.contains(&key) {
            let error_msg = format!(
                "CommandRouter: Already subscribed to command type {} (routing key: {:?})",
                target_descriptor, key
            );
            println!("Warning: {}", error_msg);
            return Err(Error::Internal(error_msg));
        }

        let mut receivers_map = self.receivers.lock().await;

        match receivers_map.remove(&key) {
            Some(boxed_receiver) => match boxed_receiver.downcast::<mpsc::Receiver<C>>() {
                Ok(concrete_boxed_recv) => {
                    subscribed_receivers.insert(key);
                    Ok(*concrete_boxed_recv)
                }
                Err(_orig_boxed_recv) => Err(Error::Internal(format!(
                    "CommandRouter: Type mismatch for stored receiver for command type {} (routing key: {:?})",
                    target_descriptor, key
                ))),
            },
            None => Err(Error::Internal(format!(
                "CommandRouter: No receiver registered for command type {} (routing key: {:?})",
                std::any::type_name::<C>(),
                key
            ))),
        }
    }

    fn get_routing_key<C: Any + Send + 'static>(
        &self,
        target_instance_id: Option<Uuid>,
    ) -> Result<(RoutingKey, String), Error> {
        let cmd_type_name = std::any::type_name::<C>();
        let (key, target_descriptor) = match target_instance_id {
            Some(uuid) => (
                RoutingKey::Instanced(TypeId::of::<C>(), uuid),
                format!("instance({})::{}", uuid, cmd_type_name),
            ),
            None => (
                RoutingKey::Singleton(TypeId::of::<C>()),
                format!("singleton::{}", cmd_type_name),
            ),
        };

        Ok((key, target_descriptor))
    }
}
