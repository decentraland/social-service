#[cfg(test)]
mod database_tests {
    use social_service::{
        components::{
            configuration::Database,
            database::{DatabaseComponent, DatabaseComponentImplementation},
        },
        entities::friendships::FriendshipRepositoryImplementation,
    };
    use sqlx::{postgres::PgArguments, query::Query, Postgres};

    use crate::helpers::server::get_configuration;

    // TODO: Use the other method
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
        let dbrepos = db.get_repos().as_ref().unwrap();

        dbrepos
            .friendships
            .create_new_friendships(("A", "B"), None)
            .await
            .0
            .unwrap();
        let friendship = dbrepos.friendships.get(("A", "B"), None).await.0.unwrap();
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
            .create_new_friendships(("C", "D"), None)
            .await
            .0
            .unwrap();
        let friendship = dbrepos
            .friendships
            .get(("C", "D"), None)
            .await
            .0
            .unwrap()
            .unwrap();
        dbrepos
            .friendship_history
            .create(friendship.id, "request", "C", None, None)
            .await
            .0
            .unwrap();
        let friendship_history = dbrepos
            .friendship_history
            .get(friendship.id, None)
            .await
            .0
            .unwrap();
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
        let addresses_2 = ("2", "3");

        let trans = db.start_transaction().await.unwrap();

        let (_res, trans) = dbrepos
            .get_friendships()
            .create_new_friendships(addresses, Some(trans))
            .await;

        let (_res, trans) = dbrepos
            .get_friendships()
            .create_new_friendships(addresses_2, trans)
            .await;

        // Read from pre transaction status
        let (read, _) = dbrepos.get_friendships().get(addresses, None).await;

        match read {
            Ok(read) => {
                assert!(read.is_none())
            }
            Err(err) => panic!("Failed while reading from db {}", err),
        }

        let (read, trans) = dbrepos.get_friendships().get(addresses, trans).await;

        match read {
            Ok(read) => {
                assert!(read.is_some())
            }
            Err(err) => panic!("Failed while reading from db {}", err),
        }

        trans.unwrap().commit().await.unwrap();

        let (read, _) = dbrepos.get_friendships().get(addresses, None).await;

        match read {
            Ok(read) => {
                assert!(read.is_some())
            }
            Err(err) => panic!("Failed while reading from db {}", err),
        }
    }
}
