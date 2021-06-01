use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DiscordChannelId(u64);
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DiscordMessageId(u64);
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DiscordUserId(u64);
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct DiscordUserIdVec(Vec<DiscordUserId>);
#[derive(Debug, Serialize, Deserialize)]
struct PersistedGroupPicSession {
    join_msg_id: DiscordMessageId,
    participant_id_list: DiscordUserIdVec,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_sled() {
        let db = sled::open("my_db").unwrap();
        // insert and get
        db.insert(b"yo!", b"v1").unwrap();
        assert_eq!(&db.get(b"yo!").unwrap().unwrap(), b"v1");
        std::fs::remove_dir_all("my_db").unwrap();
    }

    #[test]
    fn persist_channel_id_and_join_msg_id() {
        let db = sled::open("group_pic").unwrap();
        let channel_id = DiscordChannelId(1);
        let join_msg_id = DiscordMessageId(1);
        let participant_id_list = DiscordUserIdVec(vec![DiscordUserId(1), DiscordUserId(2)]);
        let persisted_session = PersistedGroupPicSession {
            join_msg_id,
            participant_id_list,
        };
        db.insert(
            bincode::serialize(&channel_id).unwrap(),
            bincode::serialize(&persisted_session).unwrap(),
        )
        .unwrap();
        let recovered_session: PersistedGroupPicSession = bincode::deserialize(
            &db.get(bincode::serialize(&channel_id).unwrap())
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(recovered_session.join_msg_id, persisted_session.join_msg_id);
        assert_eq!(
            recovered_session.participant_id_list,
            persisted_session.participant_id_list
        );
        std::fs::remove_dir_all("group_pic").unwrap();
    }
}
