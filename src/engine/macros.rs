#[macro_export]
macro_rules! queue_message {
    ($commands:ident, $messages:ident, $entity:expr, $message:expr) => {
        match $messages.get_mut($entity) {
            Ok(mut messages) => messages.queue($message),
            Err(_) => {
                $commands
                    .entity($entity)
                    .insert(crate::world::Messages::new_with($message));
            }
        }
    };
}
