use dashmap::DashMap;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group, hook},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::{
    async_trait,
    model::id::{ChannelId, MessageId},
    prelude::TypeMapKey,
};

use std::{env, sync::Arc, unimplemented};
use tracing::{error, info};
use tracing_subscriber;

mod util;
use util::*;

/// Map of channels with a group pic session active to
/// the pair of join message and list of participants message
struct GroupPicSessions;

impl TypeMapKey for GroupPicSessions {
    type Value = Arc<DashMap<ChannelId, (MessageId, MessageId)>>;
}

#[group]
#[commands(ping, avatar, nick, react, grouppicbegin, grouppicend, grouppiccancel)]
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        .with_max_level(tracing::Level::INFO)
        // sets this to be the default, global collector for this application.
        .init();

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("~")) // set the bot's prefix to "~"
        .before(log_command_user)
        .group(&GENERAL_GROUP);

    // Login with a bot token from the environment
    let token = env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN env var not found, cannot log in");
    let mut client = Client::builder(token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Error creating client");

    // Initialize the set of channels with a group pic session active
    // and the map of messages with join reaction to messages of list of participants
    // enclosed in a block to drop the lock asap
    {
        let mut data = client.data.write().await;
        data.insert::<GroupPicSessions>(Arc::new(DashMap::new()));
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("An error occurred while running the client: {:?}", why);
    }
}

#[hook]
#[instrument(level = "debug")]
async fn log_command_user(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Got command '{}' by user '{}'",
        command_name, msg.author.name
    );

    true
}

/// Reply to command "Pong!"
#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.reply(ctx, "Pong!").await {
        error!("Error sending message {:?}", why)
    }

    Ok(())
}

/// Reply to command the sender's static avatar
#[command]
async fn avatar(ctx: &Context, msg: &Message) -> CommandResult {
    if let Err(why) = msg.reply(ctx, msg.author.static_face()).await {
        error!("Error sending message {:?}", why)
    }

    Ok(())
}

/// Show nickname of command sender in a following message, not reply
#[command]
async fn nick(ctx: &Context, msg: &Message) -> CommandResult {
    let nick = msg
        .author_nick(ctx)
        .await
        .unwrap_or(msg.author.name.clone());
    let content = format!("In response to message of {}", nick);
    if let Err(why) = msg.channel_id.say(ctx, content).await {
        error!("Error sending message {:?}", why)
    }
    Ok(())
}

/// Reply to command and add a reaction to the reply
#[command]
async fn react(ctx: &Context, msg: &Message) -> CommandResult {
    let content = "See the reaction below".to_string();
    match msg.reply(ctx, content).await {
        Ok(m) => {
            if let Err(why) = m.react(ctx, '📷').await {
                error!("Error reacting to message {:?}", why)
            }
        }
        Err(why) => error!("Error sending message {:?}", why),
    }
    Ok(())
}

async fn group_pic_sessions(ctx: &Context) -> Arc<DashMap<ChannelId, (MessageId, MessageId)>> {
    let data = ctx.data.read().await;
    let sessions = data.get::<GroupPicSessions>().unwrap().clone();
    
    sessions
}

/// Create a group picture session
/// 
/// Replies to the command message with two messages:
/// 1. a message with a camera reaction
/// 2. a message with a list of participants
///
/// A user can click on the camera reaction to become a participant.
/// The nickname of the user is appended to the List of participants.
#[command]
async fn grouppicbegin(ctx: &Context, msg: &Message) -> CommandResult {
    let group_pic_sessions = group_pic_sessions(ctx).await;
    // if a session is already active in the channel
    if let Some(session) = group_pic_sessions.get(&msg.channel_id) {
        let (join_msg_id, _) = *session;
        // find the join message in the channel
        match msg.channel_id.message(ctx, join_msg_id).await {
            // if found, reply with the link and return
            Ok(join_msg) => {
                let content = format!(
                    "A group picture session is already active in this channel at {}",
                    join_msg.link()
                );
                if let Err(why) = msg.reply(ctx, content).await {
                    error!("Error sending message: {:?}", why)
                }
                return Ok(());
            }
            // if not found, what's the reason?
            // the message may have been deleted by the mod.
            // it may also just be a network problem.
            // should just report error and add an abort command
            Err(why) => {
                error!("Error finding message {:?}", why);
                let content = "A group picture session \
                is active in this channel but the join message \
                is not available. You can cancel the session with grouppiccancel.";
                if let Err(why) = msg.reply(ctx, content).await {
                    error!("Error sending message: {:?}", why);
                }
                return Ok(());
            }
        }
    }

    // start new group picture session
    match msg.channel_id.send_message(ctx, |m| {
        m.reference_message(msg);
        m.content("Join the group picture session by reacting with 📷 below");
        m.reactions(vec!['📷']);
        m
    }).await {
        Ok(m1) => {
            match msg.reply(ctx, "List of participants:").await {
                Ok(m2) => {
                    // Save the channel id, the join message and the list of participants
                    // to the map of sessions
                    group_pic_sessions.insert(msg.channel_id, (m1.id, m2.id));
                }
                Err(why) => error!("Error sending message {:?}", why),
            }
        }
        Err(why) => {
            error!("Error sending message: {:?}", why);
        }
    };

    Ok(())
}

#[command]
async fn grouppicend(ctx: &Context, msg: &Message) -> CommandResult {
    unimplemented!();
}

#[command]
async fn grouppiccancel(ctx: &Context, msg: &Message) -> CommandResult {
    // check if the channel already has a session
    // if so, cancel the session:
    // 1. remove the channel from map
    // 2. delete the join and list message
    // 3. reply with success or log failure
    if let Some((_, (join_msg_id, list_msg_id))) = group_pic_sessions(ctx).await.remove(&msg.channel_id) {
        match msg.channel_id.delete_messages(ctx, vec![join_msg_id, list_msg_id]).await {
            Ok(()) => {
                let content = "Group picture session cancelled. You can start a new session with ~grouppicbegin.";
                msg.reply(ctx, content).await.unwrap();
            }
            Err(why) => {
                error!("Error deleting message: {:?}", why);
            }
        }
    }
    Ok(())
}
