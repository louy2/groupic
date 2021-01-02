use serenity::model::user::User;

pub trait StaticFace {
    fn static_face(&self) -> String;
}

impl StaticFace for User {
    fn static_face(&self) -> String {
        self.static_avatar_url()
            .unwrap_or(self.default_avatar_url())
            .replace("webp", "png")
            .replace("1024", "128")
    }
}
