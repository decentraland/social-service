use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr};
use std::sync::Arc;

use crate::components::database::DatabaseComponent;

const TABLE: &str = "user_features";

#[derive(Clone)]
pub struct UserFeaturesRepository {
    db_connection: Arc<Option<DatabaseConnection>>,
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
    pub fn new(db: Arc<Option<DatabaseConnection>>) -> Self {
        Self { db_connection: db }
    }

    pub async fn create(
        &self,
        user: String,
        feature_name: String,
        feature_value: String,
    ) -> Result<(), DbErr> {
        let query = DatabaseComponent::get_statement(
            format!("INSERT INTO {} VALUES ($1, $2, $3)", TABLE).as_str(),
            vec![user.into(), feature_name.into(), feature_value.into()],
        );
        match self
            .db_connection
            .as_ref()
            .as_ref()
            .unwrap()
            .execute(query)
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    }

    pub async fn get_all_user_features(&self, user: String) -> Result<Option<UserFeatures>, DbErr> {
        let query = DatabaseComponent::get_statement(
            format!("SELECT * FROM {} WHERE \"user\" = $1", TABLE).as_str(),
            vec![user.clone().into()],
        );
        match self
            .db_connection
            .as_ref()
            .as_ref()
            .unwrap()
            .query_all(query)
            .await
        {
            Ok(row) => {
                if row.is_empty() {
                    Ok(None)
                } else {
                    let mut all_user_features = UserFeatures {
                        user,
                        features: Vec::new(),
                    };
                    for result in row {
                        let current_feature = UserFeature {
                            feature_name: result.try_get("", "feature_name").unwrap(),
                            feature_value: result.try_get("", "feature_value").unwrap(),
                        };
                        all_user_features.features.push(current_feature)
                    }
                    Ok(Some(all_user_features))
                }
            }
            Err(err) => Err(err),
        }
    }
}
