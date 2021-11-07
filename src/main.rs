mod alias;
mod gen_pic;
mod util;

use alias::*;
use util::*;

use std::num::NonZeroU64;

use tokio_stream::StreamExt;
use tracing::{debug, error, info};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard};
use twilight_model::application::callback::InteractionResponse;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::id::{ApplicationId, GuildId};

lazy_static::lazy_static! {
    static ref TEST_GUILD_ID: GuildId = GuildId(NonZeroU64::new(715641223972651169).unwrap());
    static ref APPLICATION_ID: ApplicationId = ApplicationId(NonZeroU64::new(794225841554325516).unwrap());
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // set up global trace collector
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();
    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        .init();

    // Login with a bot token from the environment
    let token = std::env::var("DISCORD_TOKEN").expect("Please set DISCORD_TOKEN");

    let hc = twilight_http::Client::builder()
        .token(token.clone())
        .application_id(*APPLICATION_ID)
        .build();
    let me = hc.current_user().exec().await?.model().await?;
    info!("Using Discord API as {}#{}", me.name, me.discriminator());

    let ping_command = hc
        .create_guild_command(*TEST_GUILD_ID, "ping")?
        .chat_input("Replies with pong.")?
        .exec()
        .await?
        .model()
        .await?;
    let avatar_command = hc
        .create_guild_command(*TEST_GUILD_ID, "avatar")?
        .chat_input("Replies with your avatar")?
        .exec()
        .await?
        .model()
        .await?;

    let (gc, mut events) = Shard::builder(
        token.clone(),
        Intents::GUILD_MESSAGES | Intents::GUILD_MESSAGE_REACTIONS,
    )
    .event_types(EventTypeFlags::READY | EventTypeFlags::INTERACTION_CREATE)
    .build();

    gc.start().await?;

    while let Some(event) = events.next().await {
        match event {
            Event::Ready(x) => {
                let me = x.user;
                info!(
                    "Connecting to Discord Gateway as {}#{}",
                    me.name,
                    me.discriminator()
                );
            }
            Event::InteractionCreate(x) => {
                let x = x.0;
                match x {
                    Interaction::ApplicationCommand(x) => {
                        if x.data.id == ping_command.id.unwrap() {
                            let res = twilight_util::builder::CallbackDataBuilder::new()
                                .content("Pong".into())
                                .flags(MessageFlags::EPHEMERAL)
                                .build();
                            hc.create_interaction_original(
                                x.id,
                                &x.token,
                                &InteractionResponse::ChannelMessageWithSource(res),
                            )
                            .exec()
                            .await?;
                        }
                        if x.data.id == avatar_command.id.unwrap() {
                            let res = twilight_util::builder::CallbackDataBuilder::new()
                                .content("Pong".into())
                                .flags(MessageFlags::EPHEMERAL)
                                .build();
                            hc.create_interaction_original(
                                x.id,
                                &x.token,
                                &InteractionResponse::ChannelMessageWithSource(res),
                            )
                            .exec()
                            .await?;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Ok(())
}

// /// Reply to command the sender's static avatar
// #[command]
// async fn avatar(ctx: &Context, msg: &Message) -> CommandResult {
//     if let Err(why) = msg.reply(ctx, msg.author.static_face()).await {
//         error!("Error sending message {:?}", why)
//     }

//     Ok(())
// }

// /// Show nickname of command sender in a following message, not reply
// #[command]
// async fn nick(ctx: &Context, msg: &Message) -> CommandResult {
//     let nick = msg
//         .author_nick(ctx)
//         .await
//         .unwrap_or(msg.author.name.clone());
//     let content = format!("In response to message of {}", nick);
//     if let Err(why) = msg.channel_id.say(ctx, content).await {
//         error!("Error sending message {:?}", why)
//     }
//     Ok(())
// }

// /// Reply to command and add a reaction to the reply
// #[command]
// async fn react(ctx: &Context, msg: &Message) -> CommandResult {
//     let content = "See the reaction below".to_string();
//     match msg.reply(ctx, content).await {
//         Ok(m) => {
//             if let Err(why) = m.react(ctx, '📷').await {
//                 error!("Error reacting to message {:?}", why)
//             }
//         }
//         Err(why) => error!("Error sending message {:?}", why),
//     }
//     Ok(())
// }
