use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::events::{Event, ModelUnavailableStartEvent, OperationalEvent, UnavailableStartEvent};
use crate::{Credential, CredentialId, CredentialState, EventHub, UnavailableReason};

use super::model_unavailable_queue::ModelUnavailableQueue;
use super::unavailable_queue::UnavailableQueue;

type ModelStateKey = (CredentialId, String);
type ModelStateValue = (Instant, UnavailableReason);

#[derive(Debug, Clone)]
pub enum AcquireError {
    ProviderUnknown,
    NoActiveCredentials,
}

pub struct CredentialPool {
    creds: RwLock<HashMap<CredentialId, Credential>>,
    by_provider: RwLock<HashMap<String, Vec<CredentialId>>>,
    states: Arc<RwLock<HashMap<CredentialId, CredentialState>>>,
    model_states: Arc<RwLock<HashMap<ModelStateKey, ModelStateValue>>>,
    events: EventHub,
    queue: Arc<UnavailableQueue>,
    model_queue: Arc<ModelUnavailableQueue>,
}

impl CredentialPool {
    pub fn new(events: EventHub) -> Self {
        let states = Arc::new(RwLock::new(HashMap::new()));
        let model_states = Arc::new(RwLock::new(HashMap::new()));
        let queue = Arc::new(UnavailableQueue::new());
        let model_queue = Arc::new(ModelUnavailableQueue::new());
        queue
            .clone()
            .spawn_recover_task(states.clone(), events.clone());
        model_queue
            .clone()
            .spawn_recover_task(model_states.clone(), events.clone());
        Self {
            creds: RwLock::new(HashMap::new()),
            by_provider: RwLock::new(HashMap::new()),
            states,
            model_states,
            events,
            queue,
            model_queue,
        }
    }

    pub fn events(&self) -> &EventHub {
        &self.events
    }

    pub async fn insert(&self, provider: impl Into<String>, id: CredentialId, cred: Credential) {
        let provider = provider.into();
        self.creds.write().await.insert(id, cred);
        // Avoid duplicated IDs in the provider index; insert() can be called on enable toggles.
        let mut by_provider = self.by_provider.write().await;
        let ids = by_provider.entry(provider).or_default();
        if !ids.contains(&id) {
            ids.push(id);
        }
        self.states
            .write()
            .await
            .entry(id)
            .or_insert(CredentialState::Active);
    }

    pub async fn update_credential(&self, id: CredentialId, cred: Credential) {
        self.creds.write().await.insert(id, cred);
    }

    pub async fn set_enabled(&self, provider: &str, id: CredentialId, enabled: bool) {
        if enabled {
            let mut by_provider = self.by_provider.write().await;
            let ids = by_provider.entry(provider.to_string()).or_default();
            if !ids.contains(&id) {
                ids.push(id);
            }
            drop(by_provider);

            // If the credential was never inserted before, keep state as Active.
            self.states
                .write()
                .await
                .entry(id)
                .or_insert(CredentialState::Active);
        } else {
            let mut by_provider = self.by_provider.write().await;
            if let Some(ids) = by_provider.get_mut(provider) {
                ids.retain(|x| *x != id);
            }
            let mut model_states = self.model_states.write().await;
            model_states.retain(|(cred_id, _), _| *cred_id != id);
        }
    }

    pub async fn acquire(
        &self,
        provider: &str,
    ) -> Result<(CredentialId, Credential), AcquireError> {
        let ids = {
            let guard = self.by_provider.read().await;
            guard.get(provider).cloned()
        };
        let Some(ids) = ids else {
            return Err(AcquireError::ProviderUnknown);
        };

        let states = self.states.read().await;
        let chosen = ids
            .into_iter()
            .find(|id| matches!(states.get(id), Some(CredentialState::Active)));
        drop(states);

        let Some(id) = chosen else {
            return Err(AcquireError::NoActiveCredentials);
        };
        let cred = self
            .creds
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or(AcquireError::NoActiveCredentials)?;
        Ok((id, cred))
    }

    pub async fn acquire_for_model(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<(CredentialId, Credential), AcquireError> {
        let ids = {
            let guard = self.by_provider.read().await;
            guard.get(provider).cloned()
        };
        let Some(ids) = ids else {
            return Err(AcquireError::ProviderUnknown);
        };

        let states = self.states.read().await;
        let model_states = self.model_states.read().await;
        let chosen = ids.into_iter().find(|id| {
            if !matches!(states.get(id), Some(CredentialState::Active)) {
                return false;
            }
            let key = (*id, model.to_string());
            match model_states.get(&key) {
                Some((until, _reason)) => *until <= Instant::now(),
                None => true,
            }
        });
        drop(model_states);
        drop(states);

        let Some(id) = chosen else {
            return Err(AcquireError::NoActiveCredentials);
        };
        let cred = self
            .creds
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or(AcquireError::NoActiveCredentials)?;
        Ok((id, cred))
    }

    pub async fn mark_unavailable(
        &self,
        credential_id: CredentialId,
        duration: Duration,
        reason: UnavailableReason,
    ) {
        let until_instant = Instant::now() + duration;
        {
            let mut guard = self.states.write().await;
            guard.insert(
                credential_id,
                CredentialState::Unavailable {
                    until: until_instant,
                    reason,
                },
            );
        }
        self.queue.push(until_instant, credential_id).await;

        let until_wall = SystemTime::now()
            .checked_add(duration)
            .unwrap_or_else(SystemTime::now);
        self.events
            .emit(Event::Operational(OperationalEvent::UnavailableStart(
                UnavailableStartEvent {
                    at: SystemTime::now(),
                    credential_id,
                    reason,
                    until: until_wall,
                },
            )))
            .await;
    }

    pub async fn mark_model_unavailable(
        &self,
        credential_id: CredentialId,
        model: impl Into<String>,
        duration: Duration,
        reason: UnavailableReason,
    ) {
        let model = model.into();
        let until_instant = Instant::now() + duration;
        {
            let mut guard = self.model_states.write().await;
            guard.insert((credential_id, model.clone()), (until_instant, reason));
        }
        self.model_queue
            .push(until_instant, credential_id, model.clone())
            .await;

        let until_wall = SystemTime::now()
            .checked_add(duration)
            .unwrap_or_else(SystemTime::now);
        self.events
            .emit(Event::Operational(OperationalEvent::ModelUnavailableStart(
                ModelUnavailableStartEvent {
                    at: SystemTime::now(),
                    credential_id,
                    model,
                    reason,
                    until: until_wall,
                },
            )))
            .await;
    }

    pub async fn state(&self, credential_id: CredentialId) -> Option<CredentialState> {
        self.states.read().await.get(&credential_id).cloned()
    }

    pub async fn model_states(
        &self,
        credential_id: CredentialId,
    ) -> Vec<(String, Instant, UnavailableReason)> {
        let now = Instant::now();
        let guard = self.model_states.read().await;
        let mut rows = Vec::new();
        for ((id, model), (until, reason)) in guard.iter() {
            if *id != credential_id {
                continue;
            }
            if *until <= now {
                continue;
            }
            rows.push((model.clone(), *until, *reason));
        }
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    }
}
