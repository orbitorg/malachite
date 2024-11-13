use std::time::Duration;

use ractor::message::Message;
use ractor::ActorRef;

pub async fn ticker<Msg>(interval: Duration, target: ActorRef<Msg>, msg: impl Fn() -> Msg)
where
    Msg: Message,
{
    let mut interval = tokio::time::interval(interval);

    loop {
        interval.tick().await;

        if let Err(e) = target.cast(msg()) {
            tracing::error!(?e, ?target, "Failed to send tick message");
            break;
        }
    }
}
