# Discord Group Pic Bot

A bot that takes a "group picture" for an gathering on Discord.

Inspired by the Love Live! Discord community.

## Usage / Flow of operation

On a `/grouppicbegin` command from a moderator, the bot sends a message with a `UO` emote reaction, followed by a message containing the list of participants. It is recommended that these two messages be pinned to the channel. 

Participants can click on the reaction to join the group picture. 

On each participant's click on the reaction:
- the participant receives a confirmation message in private message. 
- the nickname of the participant is added to the end of the list of participants. 

When the moderator decides to take the picture, the moderator sends a `/grouppicend` command in the same channel.

On `/grouppicend` command:
- the message with the `UO` reaction is deleted, preventing participants from joining. 
- the list of participants kept for posterity. 
- the static avatar of each participant is stitched into a single group picture, with the title of the gathering on top. The group picture is sent to the channel the command is sent in as well as the private message of each participant.

## Use of Discord API

The intent identity is necessary for accessing user avatars.

## Details about how to generate the group picture

Each participant's avatar is downloaded as a 128x128 png file. The group picture consists of a header of the gathering title, followed by however many rows of 10-avatar rows.

10 avatars being 1280px wide, with 10px in between and 20px on the sides, 130px spacing in total, the group picture is 1410px wide.

The header is 40px tall.