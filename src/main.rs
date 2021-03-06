mod alias;
mod gen_pic;
mod util;

use anyhow::Context;
use futures::future::try_join_all;
use hyper::body::HttpBody;
use tempdir::TempDir;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::task::spawn_blocking;
use tokio_stream::StreamExt;
use tracing::{error, info};

use alias::*;
use twilight_http::request::AttachmentFile;
use util::*;

use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Event, EventTypeFlags, Intents, Shard};
use twilight_model::application::callback::InteractionResponse;
use twilight_model::application::command::{
    self, ChannelCommandOptionData, Command, CommandOption, CommandType, NumberCommandOptionData,
};
use twilight_model::application::interaction::application_command::CommandOptionValue;
use twilight_model::application::interaction::Interaction;
use twilight_model::channel::message::MessageFlags;
use twilight_model::channel::{Channel, ChannelType, GuildChannel};
use twilight_model::guild::Member;
use twilight_model::id::{marker::ApplicationMarker, Id};
use twilight_util::builder::command::CommandBuilder;

type ApplicationId = Id<ApplicationMarker>;

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
    let token = std::env::var("DISCORD_BOT_TOKEN").expect("Please set DISCORD_BOT_TOKEN");

    let application_id = std::env::var("DISCORD_APP_ID").expect("Please set DISCORD_APP_ID");
    let application_id = application_id.parse::<u64>()?;
    let application_id = ApplicationId::new_checked(application_id)
        .ok_or(anyhow::anyhow!("Invalid application id in DISCORD_APP_ID"))?;

    let hc = twilight_http::Client::builder()
        .token(token.clone())
        .build();
    let me = hc.current_user().exec().await?.model().await?;
    info!("Using Discord API as {}#{}", me.name, me.discriminator());
    let ic = hc.interaction(application_id);

    let commands = ic
        .set_global_commands(&[
            CommandBuilder::new(
                "ping".into(),
                "Replies with pong".into(),
                CommandType::ChatInput,
            )
            .build(),
            CommandBuilder::new(
                "avatar".into(),
                "Replies with your avatar".into(),
                CommandType::ChatInput,
            )
            .build(),
            CommandBuilder::new(
                "groupic".into(),
                "Replies with a group picture of the given voice channel".into(),
                CommandType::ChatInput,
            )
            .option(CommandOption::Channel(ChannelCommandOptionData {
                channel_types: vec![ChannelType::GuildVoice],
                description: "The voice channel for group picture".into(),
                name: "channel".into(),
                required: true,
            }))
            .option(CommandOption::Integer(NumberCommandOptionData {
                choices: vec![],
                min_value: Some(command::CommandOptionValue::Integer(5)),
                max_value: Some(command::CommandOptionValue::Integer(20)),
                description: "Number of avatars in a row / number of columns".into(),
                name: "column-count".into(),
                required: false,
                autocomplete: false,
            }))
            .build(),
        ])
        .exec()
        .await?
        .models()
        .await?;

    let ping_command: &Command = commands.get(0).unwrap();
    info!(
        "Command /ping registered with id {}",
        ping_command.id.unwrap()
    );
    let avatar_command: &Command = commands.get(1).unwrap();
    info!(
        "Command /avatar registered with id {}",
        avatar_command.id.unwrap()
    );
    let groupic_command: &Command = commands.get(2).unwrap();
    info!(
        "Command /groupic registered with id {}",
        groupic_command.id.unwrap()
    );

    let (gc, mut events) = Shard::builder(
        token.clone(),
        Intents::GUILDS
            | Intents::GUILD_MESSAGES
            | Intents::GUILD_MESSAGE_REACTIONS
            | Intents::GUILD_VOICE_STATES,
    )
    .event_types(
        EventTypeFlags::GUILDS
            | EventTypeFlags::INTERACTION_CREATE
            | EventTypeFlags::GUILD_VOICE_STATES
            | EventTypeFlags::VOICE_STATE_UPDATE,
    )
    .build();
    gc.start().await?;

    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::GUILD | ResourceType::VOICE_STATE)
        .build();

    while let Some(event) = events.next().await {
        cache.update(&event);
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
                    Interaction::ApplicationCommand(ac)
                        if ac.data.id == ping_command.id.unwrap() => {}
                    Interaction::ApplicationCommand(ac) => {
                        // dispatch to ping
                        if ac.data.id == ping_command.id.unwrap() {
                            let res = twilight_util::builder::CallbackDataBuilder::new()
                                .content("Pong".into())
                                .flags(MessageFlags::EPHEMERAL)
                                .build();
                            ic.create_interaction_original(
                                ac.id,
                                &ac.token,
                                &InteractionResponse::ChannelMessageWithSource(res),
                            )
                            .exec()
                            .await?;
                        }
                        // dispatch to avatar
                        if ac.data.id == avatar_command.id.unwrap() {
                            let avatar_url: String = match ac.member {
                                Some(m) => match m.avatar {
                                    // get guild member avatar if exists
                                    Some(member_avatar) => {
                                        if !matches!((&ac.guild_id, &m.user), (Some(_), Some(_))) {
                                            error!("Gateway event INTERACTION_CREATE should have guild_id and member.user but doesn't");
                                            continue;
                                        } else {
                                            cdn::get_guild_member_avatar(
                                                ac.guild_id.unwrap(),
                                                m.user.unwrap().id,
                                                member_avatar,
                                                cdn::PJWG::PNG,
                                            )
                                        }
                                    }
                                    // get user avatar otherwise
                                    None => match m.user {
                                        // get user avatar if exists
                                        Some(u) => match u.avatar {
                                            Some(user_avatar) => cdn::get_user_avatar(
                                                u.id,
                                                user_avatar,
                                                cdn::PJWG::PNG,
                                            ),
                                            None => cdn::get_default_user_avatar(u.discriminator),
                                        },
                                        // get default avatar otherwise
                                        None => {
                                            error!("Gateway event INTERACTION_CREATE should have member.user but doesn't");
                                            continue;
                                        }
                                    },
                                },
                                None => {
                                    let u = ac.user.unwrap();
                                    match u.avatar {
                                        Some(user_avatar) => {
                                            cdn::get_user_avatar(u.id, user_avatar, cdn::PJWG::PNG)
                                        }
                                        None => cdn::get_default_user_avatar(u.discriminator),
                                    }
                                }
                            };
                            let res = twilight_util::builder::CallbackDataBuilder::new()
                                .content(avatar_url)
                                .flags(MessageFlags::EPHEMERAL)
                                .build();
                            ic.create_interaction_original(
                                ac.id,
                                &ac.token,
                                &InteractionResponse::ChannelMessageWithSource(res),
                            )
                            .exec()
                            .await?;
                        }
                        // dispatch to groupic
                        if ac.data.id == groupic_command.id.unwrap() {
                            let mut options = ac.data.options;
                            let cov = options
                                .iter_mut()
                                .find(|cdo| cdo.name == "channel")
                                .unwrap()
                                .clone()
                                .value;
                            let ci = match cov {
                                CommandOptionValue::Channel(ci) => ci,
                                _ => {
                                    error!(
                                        "Should get guild voice channel but instead got {:?}",
                                        cov
                                    );
                                    continue;
                                }
                            };
                            dbg_trace!(&ci);
                            let gi = match ac.guild_id {
                                Some(gi) => gi,
                                None => {
                                    error!("Command cannot be used outside of a guild");
                                    continue;
                                }
                            };
                            let c = hc.channel(ci).exec().await?.model().await?;
                            let gc = match c {
                                Channel::Guild(gc) => gc,
                                _ => {
                                    error!(
                                        "Should get guild voice channel but instead got {:?}",
                                        c
                                    );
                                    continue;
                                }
                            };
                            let vc = match gc {
                                GuildChannel::Voice(vc) => vc,
                                _ => {
                                    error!(
                                        "Should get guild voice channel but instead got {:?}",
                                        gc
                                    );
                                    continue;
                                }
                            };

                            dbg_trace!(&gi);
                            let voice_states = match cache.voice_channel_states(ci) {
                                Some(vcss) => vcss,
                                None => {
                                    error!("Failed to get voice states for channel {}", vc.name);
                                    continue;
                                }
                            };
                            let mut v_m: Vec<_> = Vec::with_capacity(1 << 5); // 128
                            for vs in voice_states.inspect(|vs| {
                                dbg_trace!(vs.user_id);
                            }) {
                                if vs.channel_id.unwrap() == ci {
                                    match vs.member.clone() {
                                        Some(m) => {
                                            v_m.push(m);
                                        }
                                        None => {
                                            let m: Member = hc
                                                .guild_member(gi, vs.user_id)
                                                .exec()
                                                .await?
                                                .model()
                                                .await?;
                                            v_m.push(m);
                                        }
                                    }
                                }
                            }
                            dbg_trace!(&v_m);
                            

                            // download avatars to this temp dir
                            let avatars_dir = TempDir::new("avatars").unwrap();
                            let avatars_dir_path = avatars_dir.path().to_owned();
                            // construct async download tasks for each image file
                            let https = hyper_rustls::HttpsConnectorBuilder::new()
                                .with_native_roots()
                                .https_only()
                                .enable_http1()
                                .enable_http2()
                                .build();
                            let rc: hyper::Client<_, hyper::Body> =
                                hyper::Client::builder().build(https);
                            let download_futs: Vec<_> = v_m
                                .iter()
                                .map(|m| match m.avatar.as_ref() {
                                    Some(s) => cdn::get_guild_member_avatar(
                                        gi,
                                        m.user.id,
                                        s,
                                        cdn::PJWG::PNG,
                                    ),
                                    None => match m.user.avatar.as_ref() {
                                        Some(s) => {
                                            cdn::get_user_avatar(m.user.id, s, cdn::PJWG::PNG)
                                        }
                                        None => cdn::get_default_user_avatar(m.user.discriminator),
                                    },
                                })
                                .map(move |url| {
                                    let avatars_dir_path = avatars_dir_path.clone();
                                    let rc = rc.clone();
                                    async move {
                                        let uri: hyper::Uri = url.parse().unwrap();
                                        let mut file = {
                                            let fname = std::path::Path::new(uri.path())
                                                .file_name()
                                                .unwrap();
                                            fs::File::create(avatars_dir_path.join(fname))
                                                .await
                                                .unwrap()
                                        };
                                        let mut res = rc.get(uri).await?;
                                        while let Some(chunk) = res.body_mut().data().await {
                                            file.write_all(&chunk?).await?;
                                        }
                                        Result::<_, anyhow::Error>::Ok(())
                                    }
                                })
                                .collect();
                            // run downloads concurrently
                            try_join_all(download_futs).await?;

                            let groupic_path = avatars_dir.path().join("groupic.png");

                            let ad_clone = avatars_dir.path().to_owned();
                            let vn_clone = vc.name.clone();
                            let gp_clone = groupic_path.clone();
                            dbg_debug!(&avatars_dir.path().is_dir());
                            use std::convert::TryFrom;
                            let column_count = options
                                .iter_mut()
                                .find(|cdo| cdo.name == "column-count")
                                .and_then(|cdo| match cdo.value {
                                    CommandOptionValue::Integer(x) => Some(
                                        u32::try_from(x)
                                            .expect("column-count should be between 5 and 20"),
                                    ),
                                    _ => {
                                        error!("Should get integer for column-count but instead got something else");
                                        None
                                    }
                                });
                            spawn_blocking(move || {
                                gen_pic::generate_group_pic(
                                    ad_clone,
                                    gp_clone,
                                    column_count,
                                    vn_clone,
                                );
                            })
                            .await?;
                            dbg_debug!(&groupic_path.is_file());

                            // let content = vc.name
                            //     + "\n"
                            //     + &v_m
                            //         .into_iter()
                            //         .map(|m| m.nick.unwrap_or(m.user.name))
                            //         .collect::<Vec<_>>()
                            //         .join("\n");
                            let content = "Oats curry everyone!".to_owned();
                            let cbd = twilight_util::builder::CallbackDataBuilder::new()
                                .content(content)
                                // .flags(MessageFlags::EPHEMERAL)
                                .build();
                            ic.create_interaction_original(
                                ac.id,
                                &ac.token,
                                &InteractionResponse::ChannelMessageWithSource(cbd),
                            )
                            .exec()
                            .await?;
                            let groupic_bytes = fs::read(groupic_path)
                                .await
                                .with_context(|| "Failed to read groupic.png")?;
                            let af = AttachmentFile::from_bytes("groupic.png", &groupic_bytes);
                            ic.update_interaction_original(&ac.token)
                                .attach(&[af])
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
