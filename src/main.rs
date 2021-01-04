use serenity::async_trait;
use serenity::client::{Client, Context, EventHandler};
use serenity::framework::standard::{
    macros::{command, group, hook},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;

use std::env;
use tracing::{error, info};
use tracing_subscriber;

mod util;
use util::*;

#[group]
#[commands(ping, avatar, nick, react, grouppicbegin)]
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
        Err(why) => error!("Error sending message {:?}", why)
    }
    Ok(())
}

/// Create a group picture session
/// 
#[command]
async fn grouppicbegin(ctx: &Context, msg: &Message) -> CommandResult {
    let content = "Join the group picture session by reacting with 📷 below".to_string();
    match msg.reply(ctx, content).await {
        Ok(m) => {
            if let Err(why) = m.react(ctx, '📷').await {
                error!("Error reacting to message {:?}", why)
            }
        }
        Err(why) => error!("Error sending message {:?}", why)
    }
    match msg.reply(ctx, "List of participants:").await {
        Ok(m) => {
        }
        Err(why) => error!("Error sending message {:?}", why)
    }
    Ok(())
}