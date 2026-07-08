use mongodb::Database;
use std::sync::Arc;

use crate::{auth_client::AuthClient, community_client::CommunityClient, config::AppConfig, payments_client::PaymentsClient};

#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub db: Database,
    pub config: AppConfig,
    pub auth_client: AuthClient,
    pub payments_client: PaymentsClient,
    pub community_client: CommunityClient,
}

impl AppState {
    pub fn new(db: Database, config: AppConfig) -> Self {
        let auth_client = AuthClient::new(config.auth_base_url.clone());
        let payments_client = PaymentsClient::new(config.payments_base_url.clone());
        let community_client = CommunityClient::new(config.community_base_url.clone());
        AppState(Arc::new(AppStateInner {
            db,
            config,
            auth_client,
            payments_client,
            community_client,
        }))
    }
}

impl std::ops::Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
