# Discord Group Pic Bot

A bot that takes a "group picture" for an gathering on Discord.

Inspired by the Love Live! Discord community.

## Usage / Flow of operation

On a `~grouppicbegin` command from a moderator, the bot sends a message with a `camera` emote reaction, followed by a message containing the list of participants. It is recommended that these two messages be pinned to the channel. 

Participants can click on the reaction to join the group picture. 

On each participant's click on the reaction:
- the participant receives a confirmation message in private message. 
- the nickname of the participant is added to the end of the list of participants. 

When the moderator decides to take the picture, the moderator sends a `~grouppicend` command in the same channel.

On `~grouppicend` command:
- the message with the `camera` reaction is deleted, preventing participants from joining. 
- the list of participants kept for posterity. 
- the static avatar of each participant is stitched into a single group picture, with the title of the gathering on top. The group picture is sent to the channel the command is sent in as well as the private message of each participant.

## Use of Discord API

The intent identity is necessary for accessing user avatars.

The permission Send Message is necessary for sending message to a channel.

## Details about how to generate the group picture

Each participant's avatar is downloaded as a 128x128 png file. The group picture consists of a header of the gathering title, followed by however many rows of 5-avatar rows.

The header is 64px tall.

## User facing error messages

For each channel only one group picture session can be active. If `~grouppicbegin` is sent to a channel it creates a session which lasts until `~grouppicend` is sent in the same channel.

```
~grouppicbegin
*normal group picture session message*
~grouppicbegin
reply: a group picture session is already active in this channel at *session message link*
```

If `~grouppicend` is sent to a channel without an active group picture session

```
~grouppicend
reply: No group picture session active in this channel. If you'd like to create one, use `~grouppicbegin`.
```

## Recovery from failure (TODO)

The bot maintains in memory a set of channels with active group picture session and a mapping from the reaction message to the corresponding list of participants message. In case of a crash or shutdown, this information should be persisted to a SQLite database.

## Image Processing

`image-rs` is used to process PNG avatar images. `rusttype` is used to layout the gathering title in the header.

## Other Utility Commands

Commands made for familiarizing with the API and debugging.

- `~ping`: reply to the command message with "Pong!"
- `~avatar`: reply to the command message with URL to the static avatar icon of the sender

## Other Learnings

- The best algorithm for scaling up: Catmull-Rom; for scaling down: Lanczos 

## References

* Structured multi-threaded logging: [tracing](https://docs.rs/tracing/0.1.25/tracing/index.htm)
* Discord API: [serenity](https://docs.rs/serenity/0.10.4/serenity/index.html)
* Embedded database: [sled](https://docs.rs/sled/0.34.6/sled/index.html)
* What is the best image downscaling algorithm (quality-wise)?: [StackOverflow](https://stackoverflow.com/a/6171860)