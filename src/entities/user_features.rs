use std::sync::Arc;

use sqlx::{Error, Row};

use crate::components::database::{DBConnection, DatabaseComponent};

#[derive(Clone)]
pub struct UserFeaturesRepository {
    db_connection: Arc<Option<DBConnection>>,
}

pub struct UserFeature {
    pub feature_name: String,
    pub feature_value: String,
}

pub struct UserFeatures {
    pub user: String,
    pub features: Vec<UserFeature>,
}

impl UserFeaturesRepository {
    pub fn new(db: Arc<Option<DBConnection>>) -> Self {
        Self { db_connection: db }
    }

    pub async fn create(
        &self,
        user: &str,
        feature_name: &str,
        feature_value: &str,
    ) -> Result<(), sqlx::Error> {
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);

        match sqlx::query("INSERT INTO user_features VALUES ($1,$2,$3)")
            .bind(user)
            .bind(feature_name)
            .bind(feature_value)
            .execute(db_conn)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get_all_user_features(
        &self,
        user: &str,
    ) -> Result<Option<UserFeatures>, sqlx::Error> {
        let db_conn = DatabaseComponent::get_connection(&self.db_connection);
        match sqlx::query("SELECT * FROM user_features WHERE \"user\" = $1")
            .bind(user)
            .fetch_all(db_conn)
            .await
        {
            Ok(row) => {
                if row.is_empty() {
                    Ok(None)
                } else {
                    let mut all_user_features = UserFeatures {
                        user: user.to_string(),
                        features: Vec::new(),
                    };
                    for result in row {
                        let current_feature = UserFeature {
                            feature_name: result.try_get("feature_name").unwrap(),
                            feature_value: result.try_get("feature_value").unwrap(),
                        };
                        all_user_features.features.push(current_feature)
                    }
                    Ok(Some(all_user_features))
                }
            }
            Err(err) => match err {
                Error::RowNotFound => Ok(None),
                _ => Err(err),
            },
        }
    }
}
