use std::sync::Arc;

use futures_util::StreamExt;

use dcl_rpc::stream_protocol::Generator;

use crate::{
    entities::friendships::FriendshipRepositoryImplementation, ws::app::SocialContext,
    FriendshipsServiceServer, Payload, RequestEvents, ServerStreamResponse,
    SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload, UpdateFriendshipResponse,
    User, Users,
};

use super::{
    errors::FriendshipsServiceError,
    friendship_event_updates::handle_friendship_update,
    mapper::{event_response_as_update_response, friendship_requests_as_request_events},
    synapse_handler::get_user_id_from_request,
};

#[derive(Debug)]
pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext, FriendshipsServiceError> for MyFriendshipsService {
    #[tracing::instrument(name = "RPC SERVER > Get Friends Generator", skip(request, context))]
    async fn get_friends(
        &self,
        request: Payload,
        context: Arc<SocialContext>,
    ) -> Result<ServerStreamResponse<Users>, FriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request,
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await?;

        let social_id = user_id.social_id.clone();
        log::info!("Getting all friends for user: {}", social_id);
        // Look for users friends
        let mut friendship = match context.db.db_repos.clone() {
            Some(repos) => {
                let friendship = repos
                    .friendships
                    .get_user_friends_stream(&user_id.social_id, true)
                    .await;
                match friendship {
                    Ok(it) => it,
                    Err(err) => {
                        log::error!("Get friends > Get user friends stream > Error: {err}.");
                        return Err(FriendshipsServiceError::InternalServerError);
                    }
                }
            }
            None => {
                log::error!("Get friends > Db repositories > `repos` is None.");
                return Err(FriendshipsServiceError::InternalServerError);
            }
        };

        let (generator, generator_yielder) = Generator::create();

        tokio::spawn(async move {
            let mut users = Users::default();
            // Map Frienships to Users
            loop {
                let friendship = friendship.next().await;
                match friendship {
                    Some(friendship) => {
                        let user: User = {
                            let address1: String = friendship.address_1;
                            let address2: String = friendship.address_2;
                            match address1.eq_ignore_ascii_case(&user_id.social_id) {
                                true => User { address: address2 },
                                false => User { address: address1 },
                            }
                        };

                        let users_len = users.users.len();

                        users.users.push(user);

                        // TODO: Move this value (5) to a Env Variable, Config or sth like that
                        if users_len == 5 {
                            generator_yielder.r#yield(users).await.unwrap();
                            users = Users::default();
                        }
                    }
                    None => {
                        generator_yielder.r#yield(users).await.unwrap();
                        break;
                    }
                }
            }
        });

        log::info!("Returning generator for all friends for user {}", social_id);
        Ok(generator)
    }

    #[tracing::instrument(name = "RPC SERVER > Get Request Events", skip(request, context))]
    async fn get_request_events(
        &self,
        request: Payload,
        context: Arc<SocialContext>,
    ) -> Result<RequestEvents, FriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request,
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await?;

        let social_id = user_id.social_id.clone();
        log::info!("Getting requests events for user: {}", social_id);
        // Look for users requests
        match context.db.db_repos.clone() {
            Some(repos) => {
                let requests = repos
                    .friendship_history
                    .get_user_pending_request_events(&user_id.social_id)
                    .await;
                match requests {
                    Ok(requests) => {
                        log::info!("Returning requests events for user {}", social_id);
                        Ok(friendship_requests_as_request_events(
                            requests,
                            user_id.social_id,
                        ))
                    }
                    Err(err) => {
                        log::error!(
                            "Get request events > Get user pending request events > Error: {err}."
                        );
                        Err(FriendshipsServiceError::InternalServerError)
                    }
                }
            }
            None => {
                log::error!("Get request events > Db repositories > `repos` is None.");
                return Err(FriendshipsServiceError::InternalServerError);
            }
        }
    }

    #[tracing::instrument(name = "RPC SERVER > Update Friendship Event", skip(request, context))]
    async fn update_friendship_event(
        &self,
        request: UpdateFriendshipPayload,
        context: Arc<SocialContext>,
    ) -> Result<UpdateFriendshipResponse, FriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let auth_token = request.clone().auth_token.take().ok_or_else(|| {
            FriendshipsServiceError::Unauthorized("`auth_token` was not provided".to_string())
        })?;
        let user_id = get_user_id_from_request(
            &auth_token,
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await?;

        // Handle friendship event update
        let friendship_update_response =
            handle_friendship_update(request.clone(), context, user_id.social_id).await?;

        // Convert event response to update response
        let update_response =
            event_response_as_update_response(request, friendship_update_response)?;

        Ok(update_response)
    }

    #[tracing::instrument(
        name = "RPC SERVER > Subscribe to friendship updates",
        skip(_request, _context)
    )]
    async fn subscribe_friendship_events_updates(
        &self,
        _request: Payload,
        _context: Arc<SocialContext>,
    ) -> Result<
        ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse>,
        FriendshipsServiceError,
    > {
        todo!()
    }
}
