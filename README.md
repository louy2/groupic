# Discord Group Pic Bot

A bot that takes a "group picture" for an gathering on Discord.

Inspired by the Love Live! Discord community.

## Use of Discord API

The intent identity is necessary for accessing user avatars.

The permission Send Message is necessary for sending the resulting picture to a channel.

## Details about how to generate the group picture

Each participant's avatar is downloaded as a 128x128 png file. The group picture consists of a header of the gathering title, followed by however many rows of 5-avatar rows.

The header is 64px tall.

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
* What is the best image downscaling algorithm (quality-wise)?: [StackOverflow](https://stackoverflow.com/a/6171860)