#[cfg(test)]
mod database_tests {
    use std::pin::Pin;

    use futures_util::Future;
    use social_service::{
        components::{configuration::Database, database::DatabaseComponent},
        routes::v1::error::CommonError,
    };
    use sqlx::{Postgres, Transaction};

    use crate::helpers::server::get_configuration;

    async fn create_db_component() -> DatabaseComponent {
        let config = get_configuration();
        let mut db = DatabaseComponent::new(&Database {
            host: config.db.host.clone(),
            name: "social_service".to_string(),
            user: config.db.user.clone(),
            password: config.db.password.clone(),
        });
        db.run().await.unwrap();
        assert!(db.is_connected());
        db
    }

    #[actix_web::test]
    #[serial_test::serial]
    async fn should_create_and_get_a_friendship() {
        let db = create_db_component().await;
        let dbrepos = db.db_repos.as_ref().unwrap();
        dbrepos
            .friendships
            .create_new_friendships(("A", "B"))
            .await
            .unwrap();
        let friendship = dbrepos.friendships.get(("A", "B")).await.unwrap();
        assert!(friendship.is_some());

        assert_eq!(friendship.as_ref().unwrap().address_1, "A");
        assert_eq!(friendship.as_ref().unwrap().address_2, "B");
    }

    #[actix_web::test]
    #[serial_test::serial]
    async fn should_create_a_friendship_request_event() {
        let db = create_db_component().await;
        let dbrepos = db.db_repos.as_ref().unwrap();
        dbrepos
            .friendships
            .create_new_friendships(("C", "D"))
            .await
            .unwrap();
        let friendship = dbrepos.friendships.get(("C", "D")).await.unwrap().unwrap();
        dbrepos
            .friendship_history
            .create(friendship.id, "request", "C", None)
            .await
            .unwrap();
        let friendship_history = dbrepos.friendship_history.get(friendship.id).await.unwrap();
        assert!(friendship_history.is_some());

        assert_eq!(
            friendship_history.as_ref().unwrap().friendship_id,
            friendship.id
        );
        assert_eq!(friendship_history.as_ref().unwrap().event, "request");
        assert_eq!(friendship_history.as_ref().unwrap().acting_user, "C");
        assert_eq!(friendship_history.as_ref().unwrap().metadata, None);
    }

    #[actix_web::test]
    #[serial_test::serial]
    async fn should_create_a_user_feature() {
        let db = create_db_component().await;
        let dbrepos = db.db_repos.as_ref().unwrap();
        dbrepos
            .user_features
            .create(
                "A".to_string(),
                "exposure_level".to_string(),
                "anyone".to_string(),
            )
            .await
            .unwrap();
        let user_features = dbrepos
            .user_features
            .get_all_user_features("A".to_string())
            .await
            .unwrap();
        assert!(user_features.is_some());
        assert_eq!(user_features.as_ref().unwrap().features.len(), 1);
        assert_eq!(
            user_features
                .as_ref()
                .unwrap()
                .features
                .get(0)
                .unwrap()
                .feature_name,
            "exposure_level"
        );
        assert_eq!(
            user_features
                .as_ref()
                .unwrap()
                .features
                .get(0)
                .unwrap()
                .feature_value,
            "anyone"
        )
    }

    #[actix_web::test]
    #[serial_test::serial]
    async fn should_run_transaction_succesfully() {
        let db = create_db_component().await;
        let dbrepos = db.db_repos.as_ref().unwrap();
        let addresses = ("1", "2");
        let mut queries: Vec<
            Box<
                dyn FnMut(
                    &Transaction<Postgres>,
                )
                    -> Pin<Box<dyn Future<Output = Result<(), CommonError>>>>,
            >,
        > = vec![];

        queries.push(Box::new(|_trans| {
            Box::pin(async move {
                dbrepos
                    .friendships
                    .create_new_friendships(addresses)
                    .await
                    .unwrap();

                Ok(())
            })
        }));

        queries.push(Box::new(|trans| {
            Box::pin(async move {
                dbrepos
                    .friendships
                    .get_user_friends(addresses.0, false, Some(&trans))
                    .await
                    .unwrap();

                Ok(())
            })
        }));
    }
}
