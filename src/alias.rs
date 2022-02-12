use twilight_http::client::InteractionClient;
use twilight_http::request::application::interaction::InteractionCallback;
use twilight_model::id::{Id, marker::InteractionMarker};
use twilight_model::application::callback::InteractionResponse;

type InteractionId = Id<InteractionMarker>;

pub trait InteractionCallbackAlias {
    fn create_interaction_original<'a>(
        &'a self,
        interaction_id: InteractionId,
        interaction_token: &'a str,
        response: &'a InteractionResponse,
    ) -> InteractionCallback<'a>;
}

impl InteractionCallbackAlias for InteractionClient<'_> {
    fn create_interaction_original<'a>(
        &'a self,
        interaction_id: InteractionId,
        interaction_token: &'a str,
        response: &'a InteractionResponse,
    ) -> InteractionCallback<'a> {
        self.interaction_callback(
            interaction_id,
            interaction_token,
            response
        )
    }
}
